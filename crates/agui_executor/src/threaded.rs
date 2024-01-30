use std::{
    future::Future,
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

use agui_sync::notify;
use futures::{
    executor::{LocalPool, LocalSpawner},
    future::{FusedFuture, RemoteHandle},
    prelude::{future::FutureExt, stream::StreamExt},
    task::LocalSpawnExt,
};
use parking_lot::Mutex;

use agui_core::{
    element::ElementId,
    engine::{
        rendering::{
            bindings::{RenderingSchedulerBinding, RenderingTask},
            RenderManager,
        },
        widgets::WidgetManager,
    },
    render::RenderObjectId,
    task::{error::TaskError, TaskHandle},
    widget::Widget,
};

use crate::EngineExecutor;

pub struct ThreadedEngineExecutor {
    widget_manager: WidgetManager<ThreadedEngineElementBinding, LocalEngineSchedulerBinding>,
    update_rx: notify::Subscriber,

    render_manager: Arc<Mutex<RenderManager<SharedEngineSchedulerBinding>>>,
    sync_tx: notify::Flag,

    spawned_rx: mpsc::Receiver<ElementId>,
    rebuilt_rx: mpsc::Receiver<ElementId>,
    forget_rx: mpsc::Receiver<ElementId>,

    pool: LocalPool,
}

impl ThreadedEngineExecutor {
    pub fn with_root(root: Widget) -> Self {
        let sync_tx = notify::Flag::new();
        let sync_rx = sync_tx.subscribe();

        let (tx, rx) = mpsc::sync_channel(0);

        std::thread::spawn({
            move || {
                let (task_tx, mut task_rx) = futures::channel::mpsc::unbounded();

                let render_tx = notify::Flag::new();
                let render_rx = render_tx.subscribe();

                let render_manager = Arc::new(Mutex::new(
                    RenderManager::builder()
                        .with_scheduler(SharedEngineSchedulerBinding { task_tx })
                        .with_notifier(render_tx)
                        .build(),
                ));

                let _ = tx.send(Arc::clone(&render_manager));

                let pool = LocalPool::default();

                let spawner = pool.spawner();

                let _ = pool.spawner().spawn_local(async move {
                    while let Some(task) = task_rx.next().await {
                        if let Ok(handle) = spawner.spawn_local_with_handle(task.task) {
                            let _ = task.reply_tx.send(handle);
                        }
                    }
                });

                ThreadedEngineRendering {
                    render_manager,

                    sync_rx,
                    render_rx,

                    pool,
                }
                .run();
            }
        });

        let render_manager = rx.recv().expect("failed to receive render manager");

        let update_tx = notify::Flag::new();
        let update_rx = update_tx.subscribe();

        let (spawned_tx, spawned_rx) = mpsc::channel();
        let (rebuilt_tx, rebuilt_rx) = mpsc::channel();
        let (forget_tx, forget_rx) = mpsc::channel();

        let pool = LocalPool::default();

        Self {
            widget_manager: WidgetManager::builder()
                .with_root(root)
                .with_element_binding(ThreadedEngineElementBinding {
                    spawned_tx,
                    rebuilt_tx,
                    forget_tx,
                })
                .with_scheduler(LocalEngineSchedulerBinding {
                    spawner: pool.spawner(),
                })
                .with_notifier(update_tx)
                .build(),
            update_rx,

            render_manager,
            sync_tx,

            spawned_rx,
            rebuilt_rx,
            forget_rx,

            pool,
        }
    }
}

impl EngineExecutor for ThreadedEngineExecutor {
    #[tracing::instrument(level = "debug", skip(self))]
    fn update(&mut self) {
        tracing::trace!("widget update started");

        let start = Instant::now();

        self.widget_manager.flush_callbacks();

        let num_iterations = self.widget_manager.update();

        let update_widget_tree_end = Instant::now();

        let mut render_manager = self.render_manager.lock();

        let lock_renderer_end = Instant::now();

        for element_id in self.forget_rx.try_iter() {
            render_manager.forget_element(element_id)
        }

        for element_id in self.spawned_rx.try_iter() {
            render_manager.on_create_element(element_id)
        }

        for element_id in self.rebuilt_rx.try_iter() {
            render_manager.on_needs_update(element_id)
        }

        if render_manager.does_need_sync() {
            render_manager.sync_render_objects(self.widget_manager.sync_data());

            self.sync_tx.notify();
        }

        let sync_render_tree_end = Instant::now();

        drop(render_manager);

        let timings = WidgetUpdateTimings {
            duration: start.elapsed(),

            update_widget_tree: update_widget_tree_end - start,

            lock_renderer: lock_renderer_end - update_widget_tree_end,
            sync_render_tree: sync_render_tree_end - lock_renderer_end,
        };

        tracing::debug!(
            ?timings,
            num_iterations = num_iterations,
            "widget update complete"
        );
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn run_until_stalled(&mut self) {
        futures::executor::block_on(async {
            'update_tree: loop {
                let update_future = self.update_rx.wait().fuse();

                self.update();

                // TODO: This should wait for the render manager to be stalled as well to match the
                // behavior of the single threaded executor.

                // Run futures until no more progress can be made and no more tree updates are
                // pending.
                loop {
                    if !self.pool.try_run_one() {
                        return;
                    }

                    if update_future.is_terminated() {
                        continue 'update_tree;
                    }
                }
            }
        })
    }

    fn run_until<Fut, Out>(mut self, fut: Fut) -> Out
    where
        Fut: Future<Output = Out>,
    {
        let fut = fut.fuse();

        futures::pin_mut!(fut);

        loop {
            let mut update_future = self.update_rx.wait().fuse();

            self.update();

            let output = self.pool.run_until(async {
                futures::select! {
                    _ = update_future => {
                        tracing::trace!("update triggered by widget notifier");
                        None
                    }

                    out = fut => {
                        Some(out)
                    }
                }
            });

            if let Some(output) = output {
                return output;
            }
        }
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn run(self) {
        self.run_until(std::future::pending::<()>())
    }
}

struct ThreadedEngineRendering {
    render_manager: Arc<Mutex<RenderManager<SharedEngineSchedulerBinding>>>,

    sync_rx: notify::Subscriber,
    render_rx: notify::Subscriber,

    pool: LocalPool,
}

impl ThreadedEngineRendering {
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn update(&mut self) {
        tracing::trace!("renderer update started");

        let start = Instant::now();

        let mut render_manager = self.render_manager.lock();

        let mut did_change = false;

        let lock_renderer_end = Instant::now();

        did_change |= render_manager.flush_layout();

        let layout_end = Instant::now();

        did_change |= render_manager.flush_paint();

        let paint_end = Instant::now();

        did_change |= render_manager.sync_views();

        let sync_views_end = Instant::now();

        let timings = RendererUpdateTimings {
            duration: start.elapsed(),

            lock_renderer: lock_renderer_end - start,
            layout: layout_end - lock_renderer_end,
            paint: paint_end - layout_end,
            sync_views: sync_views_end - paint_end,
        };

        tracing::debug!(?timings, ?did_change, "renderer update complete");
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub fn run(mut self) {
        loop {
            let mut sync_future = self.sync_rx.wait().fuse();
            let mut render_future = self.render_rx.wait().fuse();

            self.update();

            self.pool.run_until(async {
                futures::select! {
                    _ = sync_future => {
                        tracing::trace!("rendering triggered by sync notifier");
                    }

                    _ = render_future => {
                        tracing::trace!("rendering triggered by render notifier");
                    }
                }
            });
        }
    }
}

struct ThreadedEngineElementBinding {
    spawned_tx: mpsc::Sender<ElementId>,
    rebuilt_tx: mpsc::Sender<ElementId>,
    forget_tx: mpsc::Sender<ElementId>,
}

impl ElementBinding for ThreadedEngineElementBinding {
    fn on_element_spawned(&mut self, _: Option<ElementId>, id: ElementId) {
        self.spawned_tx.send(id).ok();
    }

    fn on_element_needs_rebuild(&mut self, id: ElementId) {
        self.rebuilt_tx.send(id).ok();
    }

    fn on_element_destroyed(&mut self, id: ElementId) {
        self.forget_tx.send(id).ok();
    }
}

struct LocalEngineSchedulerBinding {
    spawner: LocalSpawner,
}

impl ElementSchedulerBinding for LocalEngineSchedulerBinding {
    fn spawn_task(&self, id: ElementId, task: ElementTask) -> Result<TaskHandle<()>, TaskError> {
        tracing::trace!("spawning local task for {:?}", id);

        match self.spawner.spawn_local_with_handle(task) {
            Ok(handle) => Ok(TaskHandle::from(handle)),
            Err(_) => Err(TaskError::Shutdown),
        }
    }
}

struct SharedEngineSchedulerBinding {
    task_tx: futures::channel::mpsc::UnboundedSender<SpawnTask>,
}

impl RenderingSchedulerBinding for SharedEngineSchedulerBinding {
    fn spawn_task(
        &self,
        id: RenderObjectId,
        task: RenderingTask,
    ) -> Result<TaskHandle<()>, TaskError> {
        tracing::trace!("spawning shared task for {:?}", id);

        let (reply_tx, reply_rx) = mpsc::sync_channel(0);

        self.task_tx
            .unbounded_send(SpawnTask { task, reply_tx })
            .map_err(|_| TaskError::Shutdown)?;

        match reply_rx.recv() {
            Ok(handle) => Ok(TaskHandle::from(handle)),
            Err(_) => Err(TaskError::Shutdown),
        }
    }
}

struct SpawnTask {
    task: RenderingTask,
    reply_tx: mpsc::SyncSender<RemoteHandle<()>>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct WidgetUpdateTimings {
    duration: Duration,

    update_widget_tree: Duration,

    lock_renderer: Duration,
    sync_render_tree: Duration,
}

#[derive(Debug)]
#[allow(dead_code)]
struct RendererUpdateTimings {
    duration: Duration,

    lock_renderer: Duration,

    layout: Duration,
    paint: Duration,
    sync_views: Duration,
}

use std::{
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

use futures::{
    executor::{LocalPool, LocalSpawner},
    future::{FusedFuture, RemoteHandle},
    prelude::{future::FutureExt, stream::StreamExt},
    task::LocalSpawnExt,
};
use parking_lot::Mutex;

use crate::{
    element::ElementId,
    engine::{
        bindings::{
            ElementBinding, ElementSchedulerBinding, ElementTask, RenderingSchedulerBinding,
            RenderingTask,
        },
        rendering::RenderManager,
        update_notifier::{UpdateNotifier, UpdateReceiver},
        widgets::WidgetManager,
    },
    executor::EngineExecutor,
    task::{error::TaskError, TaskHandle},
    widget::Widget,
};

pub struct ThreadedEngineExecutor {
    widget_manager: WidgetManager<ThreadedEngineElementBinding, LocalEngineSchedulerBinding>,
    update_rx: UpdateReceiver,

    render_manager: Arc<Mutex<RenderManager<SharedEngineSchedulerBinding>>>,
    sync_tx: UpdateNotifier,

    spawned_rx: mpsc::Receiver<ElementId>,
    rebuilt_rx: mpsc::Receiver<ElementId>,
    forget_rx: mpsc::Receiver<ElementId>,

    pool: LocalPool,
}

impl ThreadedEngineExecutor {
    pub fn with_root(root: Widget) -> Self {
        let (sync_tx, sync_rx) = UpdateNotifier::new();

        let (tx, rx) = mpsc::sync_channel(1);

        std::thread::spawn({
            move || {
                let (task_tx, mut task_rx) = futures::channel::mpsc::unbounded();

                let (render_tx, render_rx) = UpdateNotifier::new();

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

        let (update_tx, update_rx) = UpdateNotifier::new();

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
        tracing::debug!("widget update started");

        let start = Instant::now();

        let num_cycles = self.widget_manager.update();

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

        render_manager.sync_render_objects(self.widget_manager.tree());

        let sync_render_tree_end = Instant::now();

        drop(render_manager);

        self.sync_tx.notify();

        let timings = WidgetUpdateTimings {
            duration: start.elapsed(),

            update_widget_tree: update_widget_tree_end - start,

            lock_renderer: lock_renderer_end - update_widget_tree_end,
            sync_render_tree: sync_render_tree_end - lock_renderer_end,
        };

        tracing::debug!(?timings, did_change = num_cycles > 0, "update complete");
    }

    #[tracing::instrument(level = "debug", skip(self))]
    fn run_until_stalled(&mut self) {
        futures::executor::block_on(async {
            'update_tree: loop {
                self.update();

                // TODO: This should wait for the render manager to be stalled as well to match the
                // behavior of the single threaded executor.

                let widget_future = self.update_rx.wait().fuse();

                futures::pin_mut!(widget_future);

                // Run futures until no more progress can be made and no more tree updates are
                // pending.
                loop {
                    if !self.pool.try_run_one() {
                        return;
                    }

                    if widget_future.is_terminated() {
                        continue 'update_tree;
                    }
                }
            }
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    fn run(mut self) {
        loop {
            self.update();

            self.pool.run_until(self.update_rx.wait());
        }
    }
}

struct ThreadedEngineRendering {
    render_manager: Arc<Mutex<RenderManager<SharedEngineSchedulerBinding>>>,

    sync_rx: UpdateReceiver,
    render_rx: UpdateReceiver,

    pool: LocalPool,
}

impl ThreadedEngineRendering {
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn update(&mut self) {
        tracing::debug!("renderer update started");

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

        let timings = RenderUpdateTimings {
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
            self.update();

            let sync_future = self.sync_rx.wait().fuse();
            let render_future = self.render_rx.wait().fuse();

            futures::pin_mut!(sync_future);
            futures::pin_mut!(render_future);

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
    fn spawn_task(&self, id: ElementId, task: RenderingTask) -> Result<TaskHandle<()>, TaskError> {
        tracing::trace!("spawning shared task for {:?}", id);

        let (reply_tx, reply_rx) = mpsc::sync_channel(1);

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
struct RenderUpdateTimings {
    duration: Duration,

    lock_renderer: Duration,

    layout: Duration,
    paint: Duration,
    sync_views: Duration,
}

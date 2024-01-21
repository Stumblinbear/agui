use std::{
    future::Future,
    pin::Pin,
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

use futures::{
    executor::{LocalPool, LocalSpawner},
    future::RemoteHandle,
    prelude::{
        future::{FusedFuture, FutureExt},
        stream::StreamExt,
    },
    task::{LocalSpawnExt, SpawnError},
};
use parking_lot::Mutex;

use crate::{
    element::ElementId,
    engine::{
        bindings::{ElementBinding, LocalSchedulerBinding, SharedSchedulerBinding},
        rendering::RenderManager,
        update_notifier::UpdateNotifier,
        widgets::WidgetManager,
    },
    executor::EngineExecutor,
    widget::Widget,
};

pub struct ThreadedEngineExecutor {
    widget_manager: WidgetManager<ThreadedEngineElementBinding, LocalEngineSchedulerBinding>,

    sync_notifier: UpdateNotifier,

    render_manager: Arc<Mutex<RenderManager<SharedEngineSchedulerBinding>>>,

    spawned_rx: mpsc::Receiver<ElementId>,
    rebuilt_rx: mpsc::Receiver<ElementId>,
    forget_rx: mpsc::Receiver<ElementId>,

    pool: LocalPool,
}

impl ThreadedEngineExecutor {
    pub fn with_root(root: Widget) -> Self {
        let sync_notifier = UpdateNotifier::new();

        let (tx, rx) = mpsc::sync_channel(1);

        std::thread::spawn({
            let sync_notifier = sync_notifier.clone();

            move || {
                let (task_tx, mut task_rx) = futures::channel::mpsc::unbounded();

                let render_manager = Arc::new(Mutex::new(
                    RenderManager::builder()
                        .with_scheduler(SharedEngineSchedulerBinding { task_tx })
                        .build(),
                ));

                tx.send(Arc::clone(&render_manager)).ok();

                let pool = LocalPool::default();

                let spawner = pool.spawner();

                pool.spawner()
                    .spawn_local(async move {
                        while let Some(task) = task_rx.next().await {
                            if let Ok(handle) = spawner.spawn_local_with_handle(task.task) {
                                task.reply_tx.send(handle).ok();
                            }
                        }
                    })
                    .ok();

                ThreadedEngineRendering {
                    render_manager,

                    sync_notifier,

                    pool,
                }
                .run();
            }
        });

        let render_manager = rx.recv().expect("failed to receive render manager");

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
                .build(),

            sync_notifier,

            render_manager,

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
        tracing::debug!("update started");

        let start = Instant::now();

        self.widget_manager.update();

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

        self.sync_notifier.notify();

        let timings = WidgetUpdateTimings {
            duration: start.elapsed(),

            update_widget_tree: update_widget_tree_end - start,

            lock_renderer: lock_renderer_end - update_widget_tree_end,
            sync_render_tree: sync_render_tree_end - lock_renderer_end,
        };

        tracing::debug!(?timings, "update complete");
    }

    #[tracing::instrument(level = "debug", skip(self))]
    fn run_until_stalled(&mut self) {
        futures::executor::block_on(async {
            'update_tree: loop {
                self.update();

                let wait = self.widget_manager.wait_for_update().fuse();

                futures::pin_mut!(wait);

                // Run futures until no more progress can be made and no more tree updates are
                // pending.
                loop {
                    if !self.pool.try_run_one() {
                        return;
                    }

                    if wait.is_terminated() {
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

            self.pool.run_until(self.widget_manager.wait_for_update());
        }
    }
}

struct ThreadedEngineRendering {
    render_manager: Arc<Mutex<RenderManager<SharedEngineSchedulerBinding>>>,

    sync_notifier: UpdateNotifier,

    pool: LocalPool,
}

impl ThreadedEngineRendering {
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn update(&mut self) {
        tracing::debug!("renderer update started");

        let start = Instant::now();

        let mut render_manager = self.render_manager.lock();

        let lock_renderer_end = Instant::now();

        render_manager.flush_layout();

        let layout_end = Instant::now();

        render_manager.flush_paint();

        let paint_end = Instant::now();

        render_manager.sync_views();

        let sync_views_end = Instant::now();

        let timings = RenderUpdateTimings {
            duration: start.elapsed(),

            lock_renderer: lock_renderer_end - start,
            layout: layout_end - lock_renderer_end,
            paint: paint_end - layout_end,
            sync_views: sync_views_end - paint_end,
        };

        tracing::debug!(?timings, "renderer update complete");
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub fn run(mut self) {
        loop {
            self.update();

            self.pool.run_until(self.sync_notifier.wait());
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

impl LocalSchedulerBinding for LocalEngineSchedulerBinding {
    fn spawn_task(
        &self,
        id: ElementId,
        future: Pin<Box<dyn Future<Output = ()> + 'static>>,
    ) -> Result<RemoteHandle<()>, SpawnError> {
        tracing::trace!("spawning local task for {:?}", id);

        self.spawner.spawn_local_with_handle(future)
    }
}

struct SharedEngineSchedulerBinding {
    task_tx: futures::channel::mpsc::UnboundedSender<SpawnTask>,
}

impl SharedSchedulerBinding for SharedEngineSchedulerBinding {
    fn spawn_task(
        &self,
        id: ElementId,
        future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    ) -> Result<RemoteHandle<()>, SpawnError> {
        tracing::trace!("spawning shared task for {:?}", id);

        let (reply_tx, reply_rx) = mpsc::sync_channel(1);

        self.task_tx
            .unbounded_send(SpawnTask {
                task: future,
                reply_tx,
            })
            .map_err(|_| SpawnError::shutdown())?;

        reply_rx.recv().map_err(|_| SpawnError::shutdown())
    }
}

struct SpawnTask {
    task: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
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

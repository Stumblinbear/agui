use std::{
    future::Future,
    pin::Pin,
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

use futures::{
    executor::{LocalPool, LocalSpawner},
    future::RemoteHandle,
    prelude::future::{FusedFuture, FutureExt},
    task::{LocalSpawnExt, SpawnError, SpawnExt},
};
use parking_lot::Mutex;

use crate::{
    element::ElementId,
    engine::{
        bindings::{ElementBinding, SchedulerBinding},
        rendering::RenderManager,
        update_notifier::UpdateNotifier,
        widgets::WidgetManager,
    },
    executor::EngineExecutor,
    widget::Widget,
};

pub struct ThreadedEngineExecutor {
    widget_manager: WidgetManager<EngineElementBinding, EngineSchedulerBinding>,

    sync_notifier: UpdateNotifier,

    render_manager: Arc<Mutex<RenderManager>>,

    spawned_rx: mpsc::Receiver<ElementId>,
    rebuilt_rx: mpsc::Receiver<ElementId>,
    forget_rx: mpsc::Receiver<ElementId>,

    pool: LocalPool,
}

impl ThreadedEngineExecutor {
    pub fn with_root(root: Widget) -> Self {
        let sync_notifier = UpdateNotifier::new();

        let render_manager = Arc::new(Mutex::new(RenderManager::default()));

        let (spawned_tx, spawned_rx) = mpsc::channel();
        let (rebuilt_tx, rebuilt_rx) = mpsc::channel();
        let (forget_tx, forget_rx) = mpsc::channel();

        std::thread::spawn({
            let sync_notifier = sync_notifier.clone();
            let render_manager = Arc::clone(&render_manager);

            move || {
                ThreadedEngineRendering {
                    sync_notifier,

                    render_manager,

                    pool: LocalPool::default(),
                }
                .run();
            }
        });

        let pool = LocalPool::default();

        Self {
            widget_manager: WidgetManager::builder()
                .with_root(root)
                .with_element_binding(EngineElementBinding {
                    spawned_tx,
                    rebuilt_tx,
                    forget_tx,
                })
                .with_scheduler(EngineSchedulerBinding {
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
    render_manager: Arc<Mutex<RenderManager>>,

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

struct EngineElementBinding {
    spawned_tx: mpsc::Sender<ElementId>,
    rebuilt_tx: mpsc::Sender<ElementId>,
    forget_tx: mpsc::Sender<ElementId>,
}

impl ElementBinding for EngineElementBinding {
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

struct EngineSchedulerBinding {
    spawner: LocalSpawner,
}

impl SchedulerBinding for EngineSchedulerBinding {
    fn spawn_local_task(
        &mut self,
        id: ElementId,
        future: Pin<Box<dyn Future<Output = ()> + 'static>>,
    ) -> Result<RemoteHandle<()>, SpawnError> {
        tracing::trace!("spawning local task for {:?}", id);

        self.spawner.spawn_local_with_handle(future)
    }

    fn spawn_shared_task(
        &mut self,
        id: ElementId,
        future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    ) -> Result<RemoteHandle<()>, SpawnError> {
        tracing::trace!("spawning shared task for {:?}", id);

        self.spawner.spawn_with_handle(future)
    }
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

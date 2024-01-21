use std::{
    future::Future,
    pin::Pin,
    sync::mpsc,
    time::{Duration, Instant},
};

use futures::{
    executor::{LocalPool, LocalSpawner},
    future::{FusedFuture, RemoteHandle},
    task::{LocalSpawnExt, SpawnError, SpawnExt},
    FutureExt,
};

use crate::{
    element::ElementId,
    engine::{
        bindings::{ElementBinding, SchedulerBinding},
        rendering::RenderManager,
        widgets::WidgetManager,
    },
    executor::EngineExecutor,
    widget::Widget,
};

pub struct LocalEngineExecutor {
    widget_manager: WidgetManager<LocalEngineElementBinding, LocalEngineSchedulerBinding>,
    render_manager: RenderManager,

    spawned_rx: mpsc::Receiver<ElementId>,
    rebuilt_rx: mpsc::Receiver<ElementId>,
    forget_rx: mpsc::Receiver<ElementId>,

    pool: LocalPool,
}

impl LocalEngineExecutor {
    pub fn with_root(root: Widget) -> Self {
        let (spawned_tx, spawned_rx) = mpsc::channel();
        let (rebuilt_tx, rebuilt_rx) = mpsc::channel();
        let (forget_tx, forget_rx) = mpsc::channel();

        let pool = LocalPool::default();

        Self {
            widget_manager: WidgetManager::builder()
                .with_root(root)
                .with_element_binding(LocalEngineElementBinding {
                    spawned_tx,
                    rebuilt_tx,
                    forget_tx,
                })
                .with_scheduler(LocalEngineSchedulerBinding {
                    spawner: pool.spawner(),
                })
                .build(),
            render_manager: RenderManager::default(),

            spawned_rx,
            rebuilt_rx,
            forget_rx,

            pool,
        }
    }
}

impl EngineExecutor for LocalEngineExecutor {
    #[tracing::instrument(level = "debug", skip(self))]
    fn update(&mut self) {
        tracing::debug!("update started");

        let start = Instant::now();

        self.widget_manager.update();

        let update_widget_tree_end = Instant::now();

        for element_id in self.forget_rx.try_iter() {
            self.render_manager.forget_element(element_id)
        }

        for element_id in self.spawned_rx.try_iter() {
            self.render_manager.on_create_element(element_id)
        }

        for element_id in self.rebuilt_rx.try_iter() {
            self.render_manager.on_needs_update(element_id)
        }

        self.render_manager
            .sync_render_objects(self.widget_manager.tree());

        let update_render_tree_end = Instant::now();

        self.render_manager.flush_layout();

        let layout_end = Instant::now();

        self.render_manager.flush_paint();

        let paint_end = Instant::now();

        self.render_manager.sync_views();

        let sync_views_end = Instant::now();

        let timings = UpdateTimings {
            duration: start.elapsed(),

            update_widget_tree: update_widget_tree_end - start,
            update_render_tree: update_render_tree_end - update_widget_tree_end,
            layout: layout_end - update_render_tree_end,
            paint: paint_end - layout_end,
            sync_views: sync_views_end - paint_end,
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

struct LocalEngineElementBinding {
    spawned_tx: mpsc::Sender<ElementId>,
    rebuilt_tx: mpsc::Sender<ElementId>,
    forget_tx: mpsc::Sender<ElementId>,
}

impl ElementBinding for LocalEngineElementBinding {
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

impl SchedulerBinding for LocalEngineSchedulerBinding {
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
struct UpdateTimings {
    duration: Duration,

    update_widget_tree: Duration,
    update_render_tree: Duration,
    layout: Duration,
    paint: Duration,
    sync_views: Duration,
}

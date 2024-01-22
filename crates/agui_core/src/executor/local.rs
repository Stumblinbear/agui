use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use futures::{
    executor::{LocalPool, LocalSpawner},
    future::FusedFuture,
    prelude::future::FutureExt,
    task::LocalSpawnExt,
};

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

pub struct LocalEngineExecutor {
    widget_manager: WidgetManager<EngineElementBinding, EngineSchedulerBinding>,
    update_rx: UpdateReceiver,

    render_manager: RenderManager<EngineSchedulerBinding>,
    render_rx: UpdateReceiver,

    spawned_rx: mpsc::Receiver<ElementId>,
    rebuilt_rx: mpsc::Receiver<ElementId>,
    forget_rx: mpsc::Receiver<ElementId>,

    pool: LocalPool,
}

impl LocalEngineExecutor {
    pub fn with_root(root: Widget) -> Self {
        let (update_tx, update_rx) = UpdateNotifier::new();
        let (render_tx, render_rx) = UpdateNotifier::new();

        let (spawned_tx, spawned_rx) = mpsc::channel();
        let (rebuilt_tx, rebuilt_rx) = mpsc::channel();
        let (forget_tx, forget_rx) = mpsc::channel();

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
                .with_notifier(update_tx)
                .build(),
            update_rx,

            render_manager: RenderManager::builder()
                .with_scheduler(EngineSchedulerBinding {
                    spawner: pool.spawner(),
                })
                .with_notifier(render_tx)
                .build(),
            render_rx,

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
        tracing::trace!("update started");

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

                let update_future = self.update_rx.wait().fuse();
                let render_future = self.render_rx.wait().fuse();

                futures::pin_mut!(update_future);
                futures::pin_mut!(render_future);

                // Run futures until no more progress can be made and no more tree updates are
                // pending.
                loop {
                    if !self.pool.try_run_one() {
                        return;
                    }

                    if update_future.is_terminated() || render_future.is_terminated() {
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

            let update_future = self.update_rx.wait().fuse();
            let render_future = self.render_rx.wait().fuse();

            futures::pin_mut!(update_future);
            futures::pin_mut!(render_future);

            self.pool.run_until(async {
                futures::select! {
                    _ = update_future => {
                        tracing::trace!("update triggered by update notifier");
                    }

                    _ = render_future => {
                        tracing::trace!("update triggered by render notifier");
                    }
                }
            });
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

impl ElementSchedulerBinding for EngineSchedulerBinding {
    fn spawn_task(&self, id: ElementId, task: ElementTask) -> Result<TaskHandle<()>, TaskError> {
        tracing::trace!("spawning local task for {:?}", id);

        match self.spawner.spawn_local_with_handle(task) {
            Ok(handle) => Ok(TaskHandle::from(handle)),
            Err(_) => Err(TaskError::Shutdown),
        }
    }
}

impl RenderingSchedulerBinding for EngineSchedulerBinding {
    fn spawn_task(&self, id: ElementId, task: RenderingTask) -> Result<TaskHandle<()>, TaskError> {
        tracing::trace!("spawning shared task for {:?}", id);

        match self.spawner.spawn_local_with_handle(task) {
            Ok(handle) => Ok(TaskHandle::from(handle)),
            Err(_) => Err(TaskError::Shutdown),
        }
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

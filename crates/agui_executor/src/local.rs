use std::{
    future::Future,
    sync::mpsc,
    time::{Duration, Instant},
};

use agui_sync::notify;
use futures::{
    executor::{LocalPool, LocalSpawner},
    future::FusedFuture,
    prelude::future::FutureExt,
    task::LocalSpawnExt,
};

use agui_core::{
    element::ElementId,
    engine::{
        rendering::{
            bindings::{RenderingSchedulerBinding, RenderingTask},
            RenderManager,
        },
        widgets::bindings::{ElementBinding, ElementSchedulerBinding, ElementTask},
        widgets::WidgetManager,
    },
    render::RenderObjectId,
    task::{error::TaskError, TaskHandle},
    widget::Widget,
};

use crate::EngineExecutor;

pub struct LocalEngineExecutor {
    widget_manager: WidgetManager<EngineElementBinding, EngineSchedulerBinding>,
    update_rx: notify::Subscriber,

    render_manager: RenderManager<EngineSchedulerBinding>,
    render_rx: notify::Subscriber,

    spawned_rx: mpsc::Receiver<ElementId>,
    rebuilt_rx: mpsc::Receiver<ElementId>,
    forget_rx: mpsc::Receiver<ElementId>,

    pool: LocalPool,
}

impl LocalEngineExecutor {
    pub fn with_root(root: Widget) -> Self {
        let update_tx = notify::Flag::new();
        let update_rx = update_tx.subscribe();

        let render_tx = notify::Flag::new();
        let render_rx = render_tx.subscribe();

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
impl LocalEngineExecutor {
    #[tracing::instrument(level = "debug", skip(self))]
    fn update_widgets(&mut self) {
        tracing::trace!("widget update started");

        let start = Instant::now();

        self.widget_manager.flush_callbacks();

        let num_iterations = self.widget_manager.update();

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

        if self.render_manager.does_need_sync() {
            self.render_manager
                .sync_render_objects(self.widget_manager.tree());
        }

        let sync_render_tree_end = Instant::now();

        let timings = WidgetUpdateTimings {
            duration: start.elapsed(),

            update_widget_tree: update_widget_tree_end - start,
            sync_render_tree: sync_render_tree_end - update_widget_tree_end,
        };

        tracing::debug!(
            ?timings,
            num_iterations = num_iterations,
            "widget update complete"
        );
    }

    #[tracing::instrument(level = "debug", skip(self))]
    fn update_renderer(&mut self) {
        tracing::trace!("renderer update started");

        let start = Instant::now();

        self.render_manager.flush_layout();

        let layout_end = Instant::now();

        self.render_manager.flush_paint();

        let paint_end = Instant::now();

        self.render_manager.sync_views();

        let sync_views_end = Instant::now();

        let timings = RendererUpdateTimings {
            duration: start.elapsed(),

            layout: layout_end - start,
            paint: paint_end - layout_end,
            sync_views: sync_views_end - paint_end,
        };

        tracing::debug!(?timings, "renderer update complete");
    }
}

impl EngineExecutor for LocalEngineExecutor {
    #[tracing::instrument(level = "debug", skip(self))]
    fn update(&mut self) {
        self.update_widgets();

        if self.render_manager.does_need_update() {
            self.update_renderer();
        }
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn run_until_stalled(&mut self) {
        'update_tree: loop {
            let update_future = self.update_rx.wait().fuse();
            let render_future = self.render_rx.wait().fuse();

            self.update();

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
    }

    fn run_until<Fut, Out>(mut self, fut: Fut) -> Out
    where
        Fut: Future<Output = Out>,
    {
        let fut = fut.fuse();

        futures::pin_mut!(fut);

        let mut update_future = self.update_rx.wait().fuse();
        let mut render_future = self.update_rx.wait().fuse();

        self.update();

        loop {
            let output = self.pool.run_until(async {
                futures::select! {
                    _ = update_future => {
                        tracing::trace!("update triggered by widget notifier");
                        None
                    }

                    _ = render_future => {
                        tracing::trace!("update triggered by render notifier");
                        None
                    }

                    output = fut => {
                        Some(output)
                    }
                }
            });

            if let Some(output) = output {
                return output;
            }

            if update_future.is_terminated() {
                self.update_widgets();
            }

            if render_future.is_terminated() || self.render_manager.does_need_update() {
                self.update_renderer();
            }

            update_future = self.update_rx.wait().fuse();
            render_future = self.render_rx.wait().fuse();
        }
    }

    #[tracing::instrument(level = "debug", skip(self))]
    fn run(self) {
        self.run_until(std::future::pending::<()>())
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
    fn spawn_task(
        &self,
        id: RenderObjectId,
        task: RenderingTask,
    ) -> Result<TaskHandle<()>, TaskError> {
        tracing::trace!("spawning shared task for {:?}", id);

        match self.spawner.spawn_local_with_handle(task) {
            Ok(handle) => Ok(TaskHandle::from(handle)),
            Err(_) => Err(TaskError::Shutdown),
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct WidgetUpdateTimings {
    duration: Duration,

    update_widget_tree: Duration,
    sync_render_tree: Duration,
}

#[derive(Debug)]
#[allow(dead_code)]
struct RendererUpdateTimings {
    duration: Duration,

    layout: Duration,
    paint: Duration,
    sync_views: Duration,
}

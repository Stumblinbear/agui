use std::{
    any::Any,
    collections::VecDeque,
    future::{Future, IntoFuture},
    sync::{mpsc, Arc},
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
    callback::{strategies::CallbackStrategy, CallbackId},
    element::{
        Element, ElementBuildContext, ElementCallbackContext, ElementId, ElementMountContext,
        ElementTaskNotifyStrategy, ElementUnmountContext, RenderObjectCreateContext,
        RenderObjectUpdateContext, RenderingTaskNotifyStrategy,
    },
    engine::{
        elements::{
            context::{ElementTreeContext, ElementTreeMountContext},
            scheduler::{CreateElementTask, ElementSchedulerStrategy},
            strategies::{InflateElementStrategy, UnmountElementStrategy},
            ElementTree,
        },
        rendering::{
            context::{RenderingLayoutContext, RenderingSpawnContext, RenderingUpdateContext},
            scheduler::{CreateRenderingTask, RenderingSchedulerStrategy},
            strategies::{
                RenderingTreeCreateStrategy, RenderingTreeLayoutStrategy,
                RenderingTreeUpdateStrategy,
            },
            RenderingTree,
        },
    },
    reactivity::{BuildError, SpawnAndInflateError},
    render::{object::RenderObject, view::View, RenderObjectId},
    task::{error::TaskError, TaskHandle},
    unit::{Constraints, Offset, Size},
    widget::{IntoWidget, Widget},
};
use rustc_hash::FxHashSet;

use crate::EngineExecutor;

pub struct LocalEngineExecutor {
    scheduler: EngineSchedulerStrategy,
    callbacks: Arc<dyn CallbackStrategy>,

    element_tree: ElementTree,

    needs_build_rx: mpsc::Receiver<ElementId>,
    rebuild_queue: VecDeque<ElementId>,

    callback_rx: mpsc::Receiver<InvokeCallback>,

    spawned_elements: VecDeque<ElementId>,
    updated_elements: VecDeque<ElementId>,

    element_update_rx: notify::Subscriber,

    rendering_tree: RenderingTree,
    render_update_rx: notify::Subscriber,

    needs_layout_rx: mpsc::Receiver<RenderObjectId>,
    needs_paint_rx: mpsc::Receiver<RenderObjectId>,

    pool: LocalPool,
}

impl Default for LocalEngineExecutor {
    fn default() -> Self {
        let (needs_build_tx, needs_build_rx) = mpsc::channel();
        let (callback_tx, callback_rx) = mpsc::channel();

        let element_update_tx = notify::Flag::new();
        let element_update_rx = element_update_tx.subscribe();

        let render_update_tx = notify::Flag::new();
        let render_update_rx = render_update_tx.subscribe();

        let (needs_layout_tx, needs_layout_rx) = mpsc::channel();
        let (needs_paint_tx, needs_paint_rx) = mpsc::channel();

        let pool = LocalPool::default();

        let scheduler = EngineSchedulerStrategy {
            needs_build_tx,

            element_update_tx: element_update_tx.clone(),

            needs_layout_tx,
            needs_paint_tx,

            render_update_tx,

            spawner: pool.spawner(),
        };

        Self {
            scheduler: scheduler.clone(),
            #[allow(clippy::arc_with_non_send_sync)]
            callbacks: Arc::new(EngineCallbackStrategy {
                callback_tx,
                element_update_tx,
            }),

            element_tree: ElementTree::default(),

            needs_build_rx,

            rebuild_queue: VecDeque::default(),

            callback_rx,

            spawned_elements: VecDeque::default(),
            updated_elements: VecDeque::default(),

            element_update_rx,

            rendering_tree: RenderingTree::default(),
            render_update_rx,

            needs_layout_rx,
            needs_paint_rx,

            pool,
        }
    }
}

impl LocalEngineExecutor {
    pub fn with_root(root: impl IntoWidget) -> Result<Self, SpawnAndInflateError<ElementId>> {
        struct InflateRootStrategy<'ctx, Sched> {
            scheduler: &'ctx mut Sched,
            callbacks: &'ctx Arc<dyn CallbackStrategy>,

            spawned_elements: &'ctx mut VecDeque<ElementId>,
        }

        impl<Sched> InflateElementStrategy for InflateRootStrategy<'_, Sched>
        where
            Sched: ElementSchedulerStrategy,
        {
            type Definition = Widget;

            fn mount(
                &mut self,
                ctx: ElementTreeMountContext,
                definition: Self::Definition,
            ) -> Element {
                self.spawned_elements.push_back(*ctx.element_id);

                let mut element = definition.create_element();

                element.mount(&mut ElementMountContext {
                    element_tree: ctx.tree,

                    parent_element_id: ctx.parent_element_id,
                    element_id: ctx.element_id,
                });

                element
            }

            fn on_forgotten(&mut self, _: ElementId) {
                unreachable!(
                    "elements should never forgotten while inflating the first root widget"
                );
            }

            fn try_update(
                &mut self,
                _: ElementId,
                _: &mut Element,
                _: &Self::Definition,
            ) -> agui_core::element::ElementComparison {
                unreachable!(
                    "elements should never be updated while inflating the first root widget"
                );
            }

            fn build(&mut self, ctx: ElementTreeContext, element: &mut Element) -> Vec<Widget> {
                let children = element.build(&mut ElementBuildContext {
                    scheduler: &mut ctx.scheduler.with_strategy(self.scheduler),
                    callbacks: self.callbacks,

                    element_tree: ctx.tree,
                    inheritance: ctx.inheritance,

                    element_id: ctx.element_id,
                });

                children
            }
        }

        let mut executor = Self::default();

        executor.element_tree.inflate(
            &mut InflateRootStrategy {
                scheduler: &mut executor.scheduler,
                callbacks: &executor.callbacks,

                spawned_elements: &mut executor.spawned_elements,
            },
            root.into_widget(),
        )?;

        Ok(executor)
    }
}

impl LocalEngineExecutor {
    #[tracing::instrument(level = "trace", skip(self))]
    fn flush_callbacks(&mut self) {
        tracing::trace!("flushing callbacks");

        while let Ok(invoke) = self.callback_rx.try_recv() {
            let element_id = invoke.callback_id.element_id();

            let existed = self
                .element_tree
                .with(invoke.callback_id.element_id(), |ctx, element| {
                    let changed = element.call(
                        &mut ElementCallbackContext {
                            scheduler: &mut ctx.scheduler.with_strategy(&mut self.scheduler),

                            element_tree: ctx.tree,
                            inheritance: ctx.inheritance,

                            element_id: &element_id,
                        },
                        invoke.callback_id,
                        invoke.arg,
                    );

                    if changed {
                        tracing::trace!("callback updated element, queueing for rebuild");

                        self.rebuild_queue.push_back(element_id);
                    }
                })
                .is_some();

            if !existed {
                tracing::warn!("callback invoked on an element that does not exist");
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn flush_needs_build(&mut self) -> bool {
        tracing::trace!("flushing needs build");

        while let Ok(element_id) = self.needs_build_rx.try_recv() {
            if self.element_tree.contains(element_id) {
                tracing::trace!(?element_id, "queueing element for rebuild");

                self.rebuild_queue.push_back(element_id);
            } else {
                tracing::warn!("queued an element for rebuild, but it does not exist in the tree");
            }
        }

        !self.rebuild_queue.is_empty()
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn flush_rebuilds(&mut self) {
        struct RebuildStrategy<'ctx, Sched> {
            scheduler: &'ctx mut Sched,
            callbacks: &'ctx Arc<dyn CallbackStrategy>,

            spawned_elements: &'ctx mut VecDeque<ElementId>,
            updated_elements: &'ctx mut VecDeque<ElementId>,

            rebuilt_elements: &'ctx mut FxHashSet<ElementId>,
        }

        impl<Sched> InflateElementStrategy for RebuildStrategy<'_, Sched>
        where
            Sched: ElementSchedulerStrategy,
        {
            type Definition = Widget;

            fn mount(
                &mut self,
                ctx: ElementTreeMountContext,
                definition: Self::Definition,
            ) -> Element {
                self.spawned_elements.push_back(*ctx.element_id);

                let mut element = definition.create_element();

                element.mount(&mut ElementMountContext {
                    element_tree: ctx.tree,

                    parent_element_id: ctx.parent_element_id,
                    element_id: ctx.element_id,
                });

                element
            }

            fn on_forgotten(&mut self, _: ElementId) {}

            fn try_update(
                &mut self,
                id: ElementId,
                element: &mut Element,
                definition: &Self::Definition,
            ) -> agui_core::element::ElementComparison {
                self.updated_elements.push_back(id);

                element.update(definition)
            }

            fn build(&mut self, ctx: ElementTreeContext, element: &mut Element) -> Vec<Widget> {
                self.rebuilt_elements.insert(*ctx.element_id);

                element.build(&mut ElementBuildContext {
                    scheduler: &mut ctx.scheduler.with_strategy(self.scheduler),
                    callbacks: self.callbacks,

                    element_tree: ctx.tree,
                    inheritance: ctx.inheritance,

                    element_id: ctx.element_id,
                })
            }
        }

        tracing::trace!("flushing rebuilds");

        // Keep track of which elements ended up being rebuilt, since build_and_realize
        // may end up rebuilding one that's currently in the queue.
        let mut rebuilt_elements = FxHashSet::default();

        rebuilt_elements.reserve(self.rebuild_queue.len().min(8));

        while let Some(element_id) = self.rebuild_queue.pop_front() {
            if rebuilt_elements.contains(&element_id) {
                tracing::trace!(
                    ?element_id,
                    "skipping element that was already rebuilt by another element"
                );

                continue;
            }

            if let Err(err) = self.element_tree.rebuild(
                &mut RebuildStrategy {
                    scheduler: &mut self.scheduler,
                    callbacks: &self.callbacks,

                    spawned_elements: &mut self.spawned_elements,
                    updated_elements: &mut self.updated_elements,

                    rebuilt_elements: &mut rebuilt_elements,
                },
                element_id,
            ) {
                match err {
                    BuildError::Broken => {
                        unreachable!("the tree is in an invalid state, aborting update");
                    }

                    BuildError::NotFound(element_id) => {
                        tracing::warn!(?element_id, "element was missing from the tree");
                    }

                    BuildError::InUse(element_id) => {
                        panic!(
                            "failed to rebuild element as it was in use: {:?}",
                            element_id
                        );
                    }
                }
            }
        }
    }

    #[tracing::instrument(level = "debug", skip(self))]
    fn update_widgets(&mut self) {
        struct SyncUnmountedStrategy<'ctx> {
            rendering_tree: &'ctx mut RenderingTree,
        }

        impl UnmountElementStrategy for SyncUnmountedStrategy<'_> {
            fn unmount(&mut self, mut ctx: ElementUnmountContext, element: Element) {
                self.rendering_tree.forget(*ctx.element_id);

                element.unmount(&mut ctx);
            }
        }

        struct SyncCreateRenderObjectStrategy<'ctx> {
            scheduler: &'ctx mut EngineSchedulerStrategy,

            element_tree: &'ctx ElementTree,

            needs_layout: &'ctx mut Vec<RenderObjectId>,
            needs_paint: &'ctx mut Vec<RenderObjectId>,
        }

        impl RenderingTreeCreateStrategy for SyncCreateRenderObjectStrategy<'_> {
            fn create(
                &mut self,
                ctx: RenderingSpawnContext,
                element_id: ElementId,
            ) -> RenderObject {
                self.element_tree
                    .as_ref()
                    .get(element_id)
                    .expect("element missing while creating render object")
                    .create_render_object(&mut RenderObjectCreateContext {
                        scheduler: &mut ctx.scheduler.with_strategy(self.scheduler),

                        render_object_id: ctx.render_object_id,
                    })
            }

            fn create_view(&mut self, element_id: ElementId) -> Option<Box<dyn View + Send>> {
                if let Element::View(view) = self
                    .element_tree
                    .as_ref()
                    .get(element_id)
                    .expect("element missing while creating view")
                {
                    Some(view.create_view())
                } else {
                    None
                }
            }

            fn mark_needs_layout(&mut self, id: RenderObjectId) {
                self.needs_layout.push(id);
            }

            fn mark_needs_paint(&mut self, id: RenderObjectId) {
                self.needs_paint.push(id);
            }
        }

        struct SyncUpdateRenderObjectStrategy<'ctx> {
            scheduler: &'ctx mut EngineSchedulerStrategy,

            element_tree: &'ctx ElementTree,

            needs_layout: &'ctx mut Vec<RenderObjectId>,
            needs_paint: &'ctx mut Vec<RenderObjectId>,
        }

        impl RenderingTreeUpdateStrategy for SyncUpdateRenderObjectStrategy<'_> {
            fn get_children(&self, element_id: ElementId) -> &[ElementId] {
                self.element_tree
                    .as_ref()
                    .get_children(element_id)
                    .expect("element missing while updating render object")
            }

            fn update(
                &mut self,
                ctx: RenderingUpdateContext,
                element_id: ElementId,
                render_object: &mut RenderObject,
            ) {
                let mut needs_layout = false;
                let mut needs_paint = false;

                self.element_tree
                    .as_ref()
                    .get(element_id)
                    .expect("element missing while updating render object")
                    .update_render_object(
                        &mut RenderObjectUpdateContext {
                            scheduler: &mut ctx.scheduler.with_strategy(self.scheduler),

                            needs_layout: &mut needs_layout,
                            needs_paint: &mut needs_paint,

                            render_object_id: ctx.render_object_id,
                        },
                        render_object,
                    );

                if needs_layout {
                    self.needs_layout.push(*ctx.render_object_id);
                } else if needs_paint {
                    self.needs_paint.push(*ctx.render_object_id);
                }
            }
        }

        tracing::trace!("widget update started");

        let start = Instant::now();

        self.flush_callbacks();

        let mut num_iterations = 0;

        // Rebuild the tree in a loop until it's fully settled. This is necessary as some
        // widgets being build may cause other widgets to be marked as dirty, which would
        // otherwise be missed in a single pass.
        while !self.rebuild_queue.is_empty() || self.flush_needs_build() {
            num_iterations += 1;

            self.flush_rebuilds();
        }

        let update_widget_tree_end = Instant::now();

        self.element_tree
            .cleanup(&mut SyncUnmountedStrategy {
                rendering_tree: &mut self.rendering_tree,
            })
            .expect("failed to cleanup element tree");

        let mut needs_layout = Vec::new();
        let mut needs_paint = Vec::new();

        for element_id in self.spawned_elements.drain(..) {
            self.rendering_tree.create(
                &mut SyncCreateRenderObjectStrategy {
                    scheduler: &mut self.scheduler,

                    element_tree: &self.element_tree,

                    needs_layout: &mut needs_layout,
                    needs_paint: &mut needs_paint,
                },
                self.element_tree.as_ref().get_parent(element_id).copied(),
                element_id,
            );
        }

        for element_id in self.updated_elements.drain(..) {
            self.rendering_tree.update(
                &mut SyncUpdateRenderObjectStrategy {
                    scheduler: &mut self.scheduler,

                    element_tree: &self.element_tree,

                    needs_layout: &mut needs_layout,
                    needs_paint: &mut needs_paint,
                },
                element_id,
            );
        }

        self.element_tree
            .cleanup(&mut SyncUnmountedStrategy {
                rendering_tree: &mut self.rendering_tree,
            })
            .expect("failed to cleanup element tree");

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
        struct SyncLayoutRenderObjectStrategy {}

        impl RenderingTreeLayoutStrategy for SyncLayoutRenderObjectStrategy {
            fn on_constraints_changed(
                &mut self,
                ctx: RenderingLayoutContext,
                constraints: Constraints,
            ) {
                println!(
                    "constraints changed: {:?} {:?}",
                    ctx.render_object_id, constraints
                );
            }

            fn on_size_changed(&mut self, ctx: RenderingLayoutContext, size: Size) {
                println!("size changed: {:?} {:?}", ctx.render_object_id, size);
            }

            fn on_offset_changed(&mut self, ctx: RenderingLayoutContext, offset: Offset) {
                println!("offset changed: {:?} {:?}", ctx.render_object_id, offset);
            }
        }

        tracing::debug!("renderer update started");

        let start = Instant::now();

        self.rendering_tree.layout_many(
            &mut SyncLayoutRenderObjectStrategy {},
            self.needs_layout_rx.try_iter(),
        );

        self.rendering_tree.sync_views();

        let layout_end = Instant::now();

        for render_object_id in self.needs_paint_rx.try_iter() {
            self.rendering_tree.paint(render_object_id);
        }

        let paint_end = Instant::now();

        self.rendering_tree.sync_views();

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

        self.update_renderer();
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn run_until_stalled(&mut self) {
        'update_tree: loop {
            let update_future = self.element_update_rx.wait().fuse();
            let render_future = self.render_update_rx.wait().fuse();

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

        let mut update_future = self.element_update_rx.wait().fuse();
        let mut render_future = self.render_update_rx.wait().fuse();

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

            if render_future.is_terminated() {
                self.update_renderer();
            }

            update_future = self.element_update_rx.wait().fuse();
            render_future = self.render_update_rx.wait().fuse();
        }
    }

    #[tracing::instrument(level = "debug", skip(self))]
    fn run(self) {
        self.run_until(std::future::pending::<()>())
    }
}

struct EngineCallbackStrategy {
    callback_tx: mpsc::Sender<InvokeCallback>,
    element_update_tx: notify::Flag,
}

impl CallbackStrategy for EngineCallbackStrategy {
    fn call_unchecked(&self, callback_id: CallbackId, arg: Box<dyn Any + Send>) {
        if let Err(err) = self.callback_tx.send(InvokeCallback { callback_id, arg }) {
            tracing::error!(?err, "failed to send callback");
        } else {
            self.element_update_tx.notify();
        }
    }
}

#[derive(Clone)]
struct EngineSchedulerStrategy {
    needs_build_tx: mpsc::Sender<ElementId>,

    element_update_tx: notify::Flag,

    needs_layout_tx: mpsc::Sender<RenderObjectId>,
    needs_paint_tx: mpsc::Sender<RenderObjectId>,

    render_update_tx: notify::Flag,

    spawner: LocalSpawner,
}

impl ElementSchedulerStrategy for EngineSchedulerStrategy {
    fn spawn_task(&mut self, task: CreateElementTask) -> Result<TaskHandle<()>, TaskError> {
        struct NotifyStrategy {
            needs_build_tx: mpsc::Sender<ElementId>,
            element_update_tx: notify::Flag,
        }

        impl ElementTaskNotifyStrategy for NotifyStrategy {
            fn mark_needs_build(&mut self, element_id: ElementId) {
                self.needs_build_tx.send(element_id).ok();
                self.element_update_tx.notify();
            }
        }

        tracing::trace!("spawning local task for {:?}", task.element_id());

        let fut = task
            .with_notify_strategy(NotifyStrategy {
                needs_build_tx: self.needs_build_tx.clone(),
                element_update_tx: self.element_update_tx.clone(),
            })
            .into_future();

        match self.spawner.spawn_local_with_handle(fut) {
            Ok(handle) => Ok(TaskHandle::from(handle)),
            Err(_) => Err(TaskError::Shutdown),
        }
    }
}

impl RenderingSchedulerStrategy for EngineSchedulerStrategy {
    fn spawn_task(&mut self, task: CreateRenderingTask) -> Result<TaskHandle<()>, TaskError> {
        struct NotifyStrategy {
            needs_layout_tx: mpsc::Sender<RenderObjectId>,
            needs_paint_tx: mpsc::Sender<RenderObjectId>,

            render_update_tx: notify::Flag,
        }

        impl RenderingTaskNotifyStrategy for NotifyStrategy {
            fn mark_needs_layout(&mut self, id: RenderObjectId) {
                self.needs_layout_tx.send(id).ok();
                self.render_update_tx.notify();
            }

            fn mark_needs_paint(&mut self, id: RenderObjectId) {
                self.needs_paint_tx.send(id).ok();
                self.render_update_tx.notify();
            }
        }

        tracing::trace!("spawning shared task for {:?}", task.render_object_id());

        let fut = task
            .with_notify_strategy(NotifyStrategy {
                needs_layout_tx: self.needs_layout_tx.clone(),
                needs_paint_tx: self.needs_paint_tx.clone(),

                render_update_tx: self.render_update_tx.clone(),
            })
            .into_future();

        match self.spawner.spawn_local_with_handle(fut) {
            Ok(handle) => Ok(TaskHandle::from(handle)),
            Err(_) => Err(TaskError::Shutdown),
        }
    }
}

#[non_exhaustive]
pub struct InvokeCallback {
    pub callback_id: CallbackId,
    pub arg: Box<dyn Any>,
}

pub struct CallError(pub Box<dyn Any>);

impl std::fmt::Debug for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CallError").field(&"...").finish()
    }
}

impl std::fmt::Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("callback channel was closed")
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

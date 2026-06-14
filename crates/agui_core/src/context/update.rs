use std::{any::Any, future::Future, rc::Rc};

use crate::{
    context::{MountCtx, TaskCtx},
    element::{BuildScope, RoutingId, RoutingTarget},
    pipeline::{
        layout::LayoutPipeline,
        paint::{PaintPipeline, PaintScope},
    },
    provide::{ProvideCell, ProvideScope},
    render_object::RenderObject,
    scheduling::{TaskHandle, TaskScheduler},
};

pub struct UpdateCtx<'a> {
    scheduler: &'a mut dyn TaskScheduler,

    routing_path: &'a mut Vec<u8>,

    provide_scope: &'a ProvideScope,
    build_scope: &'a BuildScope,

    layout: &'a LayoutPipeline,
    paint: &'a mut PaintPipeline,
    paint_scope: &'a PaintScope,
}

impl<'a> UpdateCtx<'a> {
    pub fn new(
        scheduler: &'a mut dyn TaskScheduler,
        routing_path: &'a mut Vec<u8>,
        provide_scope: &'a ProvideScope,
        build_scope: &'a BuildScope,
        layout: &'a LayoutPipeline,
        paint: &'a mut PaintPipeline,
        paint_scope: &'a PaintScope,
    ) -> Self {
        Self {
            scheduler,

            routing_path,

            provide_scope,
            build_scope,

            layout,
            paint,
            paint_scope,
        }
    }

    /// The target that addresses the current point in the build walk: its boundary and the ids within it.
    pub fn routing_target(&self) -> RoutingTarget {
        // The accumulator only ever holds whole encoded ids, so it is well-formed by construction.
        RoutingTarget::new_unchecked(
            self.build_scope.boundary().unwrap_or_default(),
            self.routing_path.clone(),
        )
    }

    pub fn provide_scope(&self) -> &ProvideScope {
        self.provide_scope
    }

    /// Reads the nearest provided value of type `T`, recording this element as a dependent so a later
    /// change to that value rebuilds it with its dependency-change hook run.
    pub fn depend_on_provided<T>(&self) -> Option<Rc<T>>
    where
        T: Any,
    {
        self.provide_scope.get_and_depend(&self.routing_target())
    }

    /// Marks the element at `target` for a dependency-change rebuild on the next flush. A [`Provide`]
    /// calls this for each reader of a value it changed.
    ///
    /// [`Provide`]: crate::provide::Provide
    pub(crate) fn mark_dependency_changed(&self, target: &RoutingTarget) {
        self.build_scope.mark_dependency_changed(target);
    }

    /// An owned scheduler handle that outlives this build.
    pub fn deferred_scheduler(&self) -> Box<dyn TaskScheduler> {
        self.scheduler.deferred()
    }

    /// Spawn a task tied to this element. `func` receives a [`TaskCtx`] it can use to post messages
    /// back to this element.
    pub fn spawn<F, Fut>(&mut self, func: F) -> Result<TaskHandle, Box<dyn std::error::Error>>
    where
        F: FnOnce(TaskCtx) -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let task_ctx = TaskCtx::new(self.scheduler.event_tx(), self.routing_target());

        self.scheduler.spawn(Box::pin(func(task_ctx)))
    }

    pub fn with_routing_id<T>(
        &mut self,
        id: RoutingId,
        func: impl FnOnce(&mut UpdateCtx) -> T,
    ) -> T {
        let mark = self.routing_path.len();
        id.encode(self.routing_path);

        let ret = func(self);

        self.routing_path.truncate(mark);

        ret
    }

    pub(crate) fn with_provided<T>(
        &mut self,
        cell: Rc<ProvideCell>,
        func: impl FnOnce(&mut UpdateCtx) -> T,
    ) -> T {
        let scope = self.provide_scope.with_cell(cell);

        let mut update_ctx = UpdateCtx {
            scheduler: self.scheduler,
            routing_path: self.routing_path,

            provide_scope: &scope,
            build_scope: self.build_scope,

            layout: self.layout,
            paint: &mut *self.paint,
            paint_scope: self.paint_scope,
        };

        func(&mut update_ctx)
    }

    /// The build boundary the current subtree is reconciled under.
    pub fn build_scope(&self) -> &BuildScope {
        self.build_scope
    }

    /// Runs `func` under `scope`, with the within-boundary routing path reset, so ids pushed inside address
    /// relative to the entered boundary rather than to the parent.
    pub fn with_build_scope<T>(
        &mut self,
        scope: &BuildScope,
        func: impl FnOnce(&mut UpdateCtx) -> T,
    ) -> T {
        let mut routing_path = Vec::new();

        let mut update_ctx = UpdateCtx {
            scheduler: self.scheduler,
            routing_path: &mut routing_path,
            provide_scope: self.provide_scope,
            build_scope: scope,

            layout: self.layout,
            paint: &mut *self.paint,
            paint_scope: self.paint_scope,
        };

        func(&mut update_ctx)
    }

    /// Reconciles a subtree with `scope` as its enclosing boundary, restoring the previous scope
    /// afterward. A boundary calls this so a render object grafted during the rebuild mounts under it.
    pub fn with_paint_scope(&mut self, scope: &PaintScope, func: impl FnOnce(&mut UpdateCtx)) {
        let mut update_ctx = UpdateCtx {
            scheduler: self.scheduler,
            routing_path: self.routing_path,

            provide_scope: self.provide_scope,
            build_scope: self.build_scope,

            paint: &mut *self.paint,
            layout: self.layout,
            paint_scope: scope,
        };

        func(&mut update_ctx);
    }

    pub fn mount<R: RenderObject + ?Sized>(&mut self, render_object: &mut R) {
        render_object.mount(&mut MountCtx::new(
            self.layout,
            self.paint,
            self.paint_scope,
        ));
    }

    pub fn unmount<R: RenderObject + ?Sized>(&mut self, render_object: &mut R) {
        render_object.unmount(&mut MountCtx::new(
            self.layout,
            self.paint,
            self.paint_scope,
        ));
    }
}

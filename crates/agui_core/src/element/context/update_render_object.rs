use std::future::Future;

use crate::{
    element::{ContextDirtyRenderObject, ElementId, RenderingTaskContext},
    engine::{rendering::scheduler::RenderingScheduler, Dirty},
    render::RenderObjectId,
    task::{context::ContextSpawnRenderingTask, error::TaskError, TaskHandle},
};

use super::{ContextElement, ContextRenderObject};

pub struct RenderObjectUpdateContext<'ctx> {
    pub scheduler: &'ctx mut RenderingScheduler<'ctx>,

    pub(crate) needs_layout: &'ctx Dirty<RenderObjectId>,
    pub(crate) needs_paint: &'ctx Dirty<RenderObjectId>,

    pub element_id: &'ctx ElementId,
    pub render_object_id: &'ctx RenderObjectId,
}

impl ContextElement for RenderObjectUpdateContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextRenderObject for RenderObjectUpdateContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}

impl ContextDirtyRenderObject for RenderObjectUpdateContext<'_> {
    fn mark_needs_layout(&mut self) {
        tracing::trace!(render_object_id = ?self.render_object_id, "render object needs layout");

        self.needs_layout.insert(*self.render_object_id);
    }

    fn mark_needs_paint(&mut self) {
        tracing::trace!(render_object_id = ?self.render_object_id, "render object needs paint");

        self.needs_paint.insert(*self.render_object_id);
    }
}

impl ContextSpawnRenderingTask for RenderObjectUpdateContext<'_> {
    fn spawn_task<Fut>(
        &mut self,
        func: impl FnOnce(RenderingTaskContext) -> Fut + 'static,
    ) -> Result<TaskHandle<()>, TaskError>
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.scheduler.spawn_task(func)
    }
}

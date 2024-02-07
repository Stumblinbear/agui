use std::future::Future;

use crate::{
    element::{ContextDirtyRenderObject, ContextRenderObject, RenderingTaskContext},
    engine::rendering::scheduler::RenderingScheduler,
    render::RenderObjectId,
    task::{context::ContextSpawnRenderingTask, error::TaskError, TaskHandle},
};

pub struct RenderObjectUpdateContext<'ctx> {
    pub scheduler: &'ctx mut RenderingScheduler<'ctx>,

    pub needs_layout: &'ctx mut bool,
    pub needs_paint: &'ctx mut bool,

    pub render_object_id: &'ctx RenderObjectId,
}

impl ContextRenderObject for RenderObjectUpdateContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}

impl ContextDirtyRenderObject for RenderObjectUpdateContext<'_> {
    fn mark_needs_layout(&mut self) {
        tracing::trace!(render_object_id = ?self.render_object_id, "render object needs layout");

        *self.needs_layout = true;
    }

    fn mark_needs_paint(&mut self) {
        tracing::trace!(render_object_id = ?self.render_object_id, "render object needs paint");

        *self.needs_paint = true;
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

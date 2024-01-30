use std::future::Future;

use crate::{
    element::{
        ContextDirtyRenderObject, ContextElement, ContextRenderObject, ElementId,
        RenderingTaskContext,
    },
    engine::rendering::scheduler::RenderingScheduler,
    render::RenderObjectId,
    task::{context::ContextSpawnRenderingTask, error::TaskError, TaskHandle},
};

pub struct RenderObjectCreateContext<'ctx> {
    pub scheduler: &'ctx mut RenderingScheduler<'ctx>,

    pub element_id: &'ctx ElementId,
    pub render_object_id: &'ctx RenderObjectId,
}

impl ContextElement for RenderObjectCreateContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextRenderObject for RenderObjectCreateContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}

impl ContextDirtyRenderObject for RenderObjectCreateContext<'_> {
    fn mark_needs_layout(&mut self) {
        // Newly created render objects will already be laid out
    }

    fn mark_needs_paint(&mut self) {
        // Newly created render objects will already be painted
    }
}

impl ContextSpawnRenderingTask for RenderObjectCreateContext<'_> {
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

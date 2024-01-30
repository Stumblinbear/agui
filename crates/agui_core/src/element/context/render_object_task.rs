use crate::{
    element::{ContextDirtyRenderObject, ContextRenderObject},
    render::RenderObjectId,
};

pub trait RenderingTaskNotifyStrategy: Send {
    fn mark_needs_layout(&mut self, render_object_id: RenderObjectId);
    fn mark_needs_paint(&mut self, render_object_id: RenderObjectId);
}

pub struct RenderingTaskContext {
    notify_strategy: Option<Box<dyn RenderingTaskNotifyStrategy>>,

    render_object_id: RenderObjectId,
}

impl RenderingTaskContext {
    pub(crate) fn new(render_object_id: RenderObjectId) -> Self {
        RenderingTaskContext {
            notify_strategy: None,
            render_object_id,
        }
    }

    pub(crate) fn with_notify_strategy<T>(self, strategy: T) -> Self
    where
        T: RenderingTaskNotifyStrategy + 'static,
    {
        Self {
            notify_strategy: Some(Box::new(strategy)),

            render_object_id: self.render_object_id,
        }
    }
}

impl ContextRenderObject for RenderingTaskContext {
    fn render_object_id(&self) -> RenderObjectId {
        self.render_object_id
    }
}

impl ContextDirtyRenderObject for RenderingTaskContext {
    fn mark_needs_layout(&mut self) {
        let Some(notify_strategy) = self.notify_strategy.as_mut() else {
            tracing::warn!(
                render_object_id = ?self.render_object_id,
                "render object needs to be laid out, but no notify strategy is set"
            );

            return;
        };

        tracing::trace!(render_object_id = ?self.render_object_id, "render object needs to be laid out");

        notify_strategy.mark_needs_layout(self.render_object_id);
    }

    fn mark_needs_paint(&mut self) {
        let Some(notify_strategy) = self.notify_strategy.as_mut() else {
            tracing::warn!(
                render_object_id = ?self.render_object_id,
                "render object needs to be painted, but no notify strategy is set"
            );

            return;
        };

        tracing::trace!(render_object_id = ?self.render_object_id, "render object needs to be painted");

        notify_strategy.mark_needs_paint(self.render_object_id);
    }
}

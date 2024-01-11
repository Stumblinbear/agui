use agui_core::{
    render::{binding::ViewBinding, canvas::Canvas, RenderObjectId},
    unit::{Offset, Size},
};

#[derive(Clone)]
pub struct VelloViewBinding;

impl VelloViewBinding {
    pub fn new() -> Self {
        Self
    }
}

impl ViewBinding for VelloViewBinding {
    fn on_attach(
        &self,
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
    ) {
        tracing::debug!(
            "VelloViewElement::on_attach {:?} {:?}",
            parent_render_object_id,
            render_object_id
        );
    }

    fn on_detach(&self, render_object_id: RenderObjectId) {
        tracing::debug!("VelloViewElement::on_detach {:?}", render_object_id);
    }

    fn on_size_changed(&self, render_object_id: RenderObjectId, size: Size) {
        tracing::debug!(
            "VelloViewElement::on_size_changed {:?} {:?}",
            render_object_id,
            size
        );
    }

    fn on_offset_changed(&self, render_object_id: RenderObjectId, offset: Offset) {
        tracing::debug!(
            "VelloViewElement::on_offset_changed {:?} {:?}",
            render_object_id,
            offset
        );
    }

    fn on_paint(&self, render_object_id: RenderObjectId, canvas: Canvas) {
        tracing::debug!(
            "VelloViewElement::on_paint {:?} {:?}",
            render_object_id,
            canvas
        );
    }

    fn on_sync(&self) {
        tracing::debug!("VelloViewElement::on_sync");
    }
}

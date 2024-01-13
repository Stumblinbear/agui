use agui_core::{
    render::{canvas::Canvas, view::View, RenderObjectId},
    unit::{Offset, Size},
};

#[derive(Clone)]
pub struct VelloView;

impl View for VelloView {
    fn on_attach(
        &mut self,
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
    ) {
        tracing::debug!(
            "VelloView::on_attach {:?} {:?}",
            parent_render_object_id,
            render_object_id
        );
    }

    fn on_detach(&mut self, render_object_id: RenderObjectId) {
        tracing::debug!("VelloView::on_detach {:?}", render_object_id);
    }

    fn on_size_changed(&mut self, render_object_id: RenderObjectId, size: Size) {
        tracing::debug!(
            "VelloView::on_size_changed {:?} {:?}",
            render_object_id,
            size
        );
    }

    fn on_offset_changed(&mut self, render_object_id: RenderObjectId, offset: Offset) {
        tracing::debug!(
            "VelloView::on_offset_changed {:?} {:?}",
            render_object_id,
            offset
        );
    }

    fn on_paint(&mut self, render_object_id: RenderObjectId, canvas: Canvas) {
        tracing::debug!("VelloView::on_paint {:?} {:?}", render_object_id, canvas);
    }

    fn on_sync(&mut self) {
        tracing::debug!("VelloView::on_sync");
    }
}

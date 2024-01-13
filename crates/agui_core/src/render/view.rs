use crate::{
    render::{canvas::Canvas, RenderObjectId},
    unit::{Offset, Size},
};

pub trait View {
    /// Called when a new render object is attached (or moved) within this element's
    /// view.
    fn on_attach(
        &mut self,
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
    );

    /// Called when a render object is detached from this element's view.
    fn on_detach(&mut self, render_object_id: RenderObjectId);

    fn on_size_changed(&mut self, render_object_id: RenderObjectId, size: Size);

    fn on_offset_changed(&mut self, render_object_id: RenderObjectId, offset: Offset);

    /// Called when the given render object within this element's view has been painted
    /// or repainted.
    fn on_paint(&mut self, render_object_id: RenderObjectId, canvas: Canvas);

    /// Called when the tree has layout and paint have been complete for this update
    /// cycle.
    fn on_sync(&mut self);

    // /// Called when a render object within this element's view updates its semantics
    // /// information.
    // fn on_needs_semantics_update(&self, render_object_id: RenderObjectId);

    // /// Called up to once per frame to redraw the view.
    // fn on_redraw(&self);

    // /// Called to render the view as it currently is.
    // ///
    // /// This is not necessarily called every frame, nor is it guaranteed to be called
    // /// after a call to [`redraw`](ElementView::redraw).
    // fn on_needs_render(&self);

    // fn load_font(&self, font_data: &[u8]) -> Result<Font, Box<dyn std::error::Error>>;

    // fn load_image(&self, image_data: &[u8]) -> Result<Texture, Box<dyn std::error::Error>>;
}

pub enum RenderView {
    Owner(Box<dyn View>),
    Within(RenderObjectId),
}

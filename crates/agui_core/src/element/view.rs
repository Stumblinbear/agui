use crate::{render::RenderObjectId, widget::Widget};

use super::widget::ElementWidget;

pub trait ElementView: ElementWidget {
    fn child(&self) -> Widget;

    /// Called when a new render object is attached (or moved) within this element's
    /// view, returning a binding that the render object may use to interact with it.
    fn on_attach(
        &mut self,
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
    );

    /// Called when a render object is detached within this element's view.
    fn on_detach(&mut self, render_object_id: RenderObjectId);

    /// Called when the given render object within this element's view was laid out.
    /// This does not necessarily mean that the render object, nor any of their children,
    /// had their size or position changed.
    ///
    /// This is only called for render objects which are considered relayout boundaries.
    fn on_layout(&mut self, render_object_id: RenderObjectId);

    /// Called when the given render object within this element's view needs to be
    /// painted.
    fn on_paint(&mut self, render_object_id: RenderObjectId);

    /// Called when a render object within this element's view needs to update its
    /// semantics information.
    fn on_needs_semantics_update(&mut self, render_object_id: RenderObjectId);

    /// Called up to once per frame to redraw the view.
    fn redraw(&mut self);

    /// Called to render the view as it currently is.
    ///
    /// This is not necessarily called every frame, nor is it guaranteed to be called
    /// after a call to [`redraw`](ElementView::redraw).
    fn render(&self);
}

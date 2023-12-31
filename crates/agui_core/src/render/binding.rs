use std::{ops::Deref, rc::Rc};

use crate::{render::RenderObjectId, util::ptr_eq::PtrEqual};

pub trait ViewBinding {
    /// Called when a new render object is attached (or moved) within this element's
    /// view, returning a binding that the render object may use to interact with it.
    fn on_attach(
        &self,
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
    );

    /// Called when a render object is detached within this element's view.
    fn on_detach(&self, render_object_id: RenderObjectId);

    /// Called when the given render object within this element's view was laid out.
    /// This does not necessarily mean that the render object, nor any of their children,
    /// had their size or position changed.
    ///
    /// This is only called for render objects which are considered relayout boundaries.
    fn on_layout(&self, render_object_id: RenderObjectId);

    /// Called when the given render object within this element's view needs to be
    /// painted.
    fn on_needs_paint(&self, render_object_id: RenderObjectId);

    /// Called when a render object within this element's view updates its semantics
    /// information.
    fn on_needs_semantics_update(&self, render_object_id: RenderObjectId);

    /// Called up to once per frame to redraw the view.
    fn on_redraw(&self);

    /// Called to render the view as it currently is.
    ///
    /// This is not necessarily called every frame, nor is it guaranteed to be called
    /// after a call to [`redraw`](ElementView::redraw).
    fn on_needs_render(&self);

    // fn load_font(&self, font_data: &[u8]) -> Result<Font, Box<dyn std::error::Error>>;

    // fn load_image(&self, image_data: &[u8]) -> Result<Texture, Box<dyn std::error::Error>>;
}

#[derive(Clone)]
pub struct RenderView {
    root_id: RenderObjectId,
    view_binding: Rc<dyn ViewBinding>,
}

impl RenderView {
    pub(crate) fn new(root_id: RenderObjectId, view_binding: Rc<dyn ViewBinding>) -> Self {
        Self {
            root_id,
            view_binding,
        }
    }

    pub fn root_id(&self) -> RenderObjectId {
        self.root_id
    }
}

impl PartialEq for RenderView {
    fn eq(&self, other: &Self) -> bool {
        self.root_id == other.root_id && self.view_binding.is_exact_ptr(&other.view_binding)
    }
}

impl Deref for RenderView {
    type Target = dyn ViewBinding;

    fn deref(&self) -> &Self::Target {
        &*self.view_binding
    }
}

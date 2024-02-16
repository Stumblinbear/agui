use crate::{
    element::{Element, ElementId},
    render::RenderObjectId,
    util::tree::Tree,
};

mod build;
mod callback;
mod create_render_object;
mod mount;
mod render_object_task;
mod task;
mod unmount;
mod update_render_object;

pub use build::*;
pub use callback::*;
pub use create_render_object::*;
pub use mount::*;
pub use render_object_task::*;
pub use task::*;
pub use unmount::*;
pub use update_render_object::*;

pub trait ContextElements {
    fn elements(&self) -> &Tree<ElementId, Element>;
}

pub trait ContextElement {
    fn element_id(&self) -> ElementId;
}

pub trait ContextRenderObject {
    fn render_object_id(&self) -> RenderObjectId;
}

pub trait ContextDirtyRenderObject {
    /// Marks the render object as needing layout.
    ///
    /// This will cause a layout pass from its nearest ancestor layout boundary.
    fn mark_needs_layout(&mut self);

    /// Marks the render object as needing paint.
    fn mark_needs_paint(&mut self);
}

pub struct ElementContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,

    pub element_id: &'ctx ElementId,
}

impl ContextElements for ElementContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for ElementContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

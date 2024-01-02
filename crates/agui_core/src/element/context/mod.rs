use crate::{
    element::{Element, ElementId},
    render::{object::RenderObject, RenderObjectId},
    util::tree::Tree,
};

mod build;
mod callback;
mod create_render_object;
mod mount;
mod unmount;
mod update_render_object;

pub use build::*;
pub use callback::*;
pub use create_render_object::*;
pub use mount::*;
pub use unmount::*;
pub use update_render_object::*;

pub trait ContextElements {
    fn elements(&self) -> &Tree<ElementId, Element>;
}

pub trait ContextElement {
    fn element_id(&self) -> ElementId;
}

pub trait ContextRenderObjects {
    fn render_objects(&self) -> &Tree<RenderObjectId, RenderObject>;
}

pub trait ContextRenderObject {
    fn render_object_id(&self) -> RenderObjectId;
}

pub trait ContextDirtyElement {
    fn mark_needs_build(&mut self);
}

pub trait ContextDirtyRenderObject {
    fn mark_needs_layout(&mut self);

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

pub struct ElementContextMut<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub element_id: &'ctx ElementId,
}

impl ContextElements for ElementContextMut<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for ElementContextMut<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

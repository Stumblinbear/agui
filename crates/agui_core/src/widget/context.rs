use crate::{
    element::{Element, ElementId},
    util::tree::Tree,
};

pub trait ContextElement {
    fn get_elements(&self) -> &Tree<ElementId, Element>;

    fn get_element_id(&self) -> ElementId;
}

pub trait ContextMarkDirty {
    fn mark_dirty(&mut self, element_id: ElementId);
}

use std::marker::PhantomData;

use fnv::FnvHashSet;

use crate::{
    element::{Element, ElementId},
    util::tree::Tree,
    widget::ContextWidget,
};

pub struct CallbackContext<'ctx, W> {
    pub(crate) phantom: PhantomData<W>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,

    pub(crate) element_id: ElementId,
}

impl<W> CallbackContext<'_, W> {
    pub fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

impl<W> ContextWidget<W> for CallbackContext<'_, W> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

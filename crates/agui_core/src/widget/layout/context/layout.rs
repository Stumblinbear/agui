use std::marker::PhantomData;

use crate::{
    element::{Element, ElementId},
    unit::Offset,
    util::tree::Tree,
    widget::{ContextWidget, ContextWidgetLayout, IterChildren, IterChildrenMut},
};

use super::ContextWidgetLayoutMut;

pub struct LayoutContext<'ctx, W> {
    pub(crate) phantom: PhantomData<W>,

    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,

    pub(crate) children: &'ctx [ElementId],
    pub(crate) offsets: &'ctx mut [Offset],
}

impl<W> ContextWidget<W> for LayoutContext<'_, W> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<'ctx, W> ContextWidgetLayout for LayoutContext<'ctx, W> {
    fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    fn child_count(&self) -> usize {
        self.children.len()
    }

    fn iter_children(&self) -> IterChildren {
        IterChildren::new(self.element_tree, self.children)
    }
}

impl<'ctx, W> ContextWidgetLayoutMut for LayoutContext<'ctx, W> {
    fn iter_children_mut(&mut self) -> IterChildrenMut {
        IterChildrenMut::new(self.element_tree, self.children, self.offsets)
    }
}

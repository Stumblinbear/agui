use std::marker::PhantomData;

use crate::{
    element::{Element, ElementId},
    util::tree::Tree,
    widget::{ContextWidget, ContextWidgetLayout, IterChildren},
};

pub struct IntrinsicSizeContext<'ctx, W> {
    pub(crate) phantom: PhantomData<W>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,

    pub(crate) children: &'ctx [ElementId],
}

impl<W> ContextWidget<W> for IntrinsicSizeContext<'_, W> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<'ctx, W> ContextWidgetLayout for IntrinsicSizeContext<'ctx, W> {
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

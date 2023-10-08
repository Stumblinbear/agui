use std::ops::{Deref, DerefMut};

use crate::{
    element::{ContextElement, Element, ElementId},
    unit::{HitTestResult, Size},
    util::tree::Tree,
};

mod iter;

pub use iter::*;

pub struct ElementHitTestContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,

    pub(crate) size: &'ctx Size,

    pub(crate) children: &'ctx [ElementId],

    pub(crate) result: &'ctx mut HitTestResult,
}

impl ContextElement for ElementHitTestContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl ElementHitTestContext<'_> {
    pub fn get_size(&self) -> Size {
        *self.size
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    pub fn iter_children(&mut self) -> IterChildrenHitTest {
        IterChildrenHitTest::new(self.element_tree, self.children, self.result)
    }
}

impl Deref for ElementHitTestContext<'_> {
    type Target = HitTestResult;

    fn deref(&self) -> &Self::Target {
        self.result
    }
}

impl DerefMut for ElementHitTestContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.result
    }
}

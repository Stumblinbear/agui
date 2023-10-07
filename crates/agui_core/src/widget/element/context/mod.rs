use std::ops::{Deref, DerefMut};

use rustc_hash::FxHashSet;

use crate::{
    element::{Element, ElementId},
    plugin::Plugins,
    unit::{HitTestResult, Offset, Size},
    util::tree::Tree,
    widget::{ContextWidget, IterChildrenHitTest, IterChildrenLayout, IterChildrenLayoutMut},
};

mod build;
mod intrinsic_size;
mod mount;
mod unmount;

pub use build::*;
pub use intrinsic_size::*;
pub use mount::*;
pub use unmount::*;

pub struct WidgetCallbackContext<'ctx> {
    pub(crate) plugins: Plugins<'ctx>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,

    pub(crate) element_id: ElementId,
}

pub struct WidgetLayoutContext<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,

    pub(crate) children: &'ctx [ElementId],
    pub(crate) offsets: &'ctx mut [Offset],
}

impl ContextWidget for WidgetLayoutContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl WidgetLayoutContext<'_> {
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    pub fn iter_children(&self) -> IterChildrenLayout {
        IterChildrenLayout::new(self.element_tree, self.children)
    }

    pub fn iter_children_mut(&mut self) -> IterChildrenLayoutMut {
        IterChildrenLayoutMut::new(self.element_tree, self.children, self.offsets)
    }
}

pub struct WidgetHitTestContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,
    pub(crate) size: &'ctx Size,

    pub(crate) children: &'ctx [ElementId],

    pub(crate) result: &'ctx mut HitTestResult,
}

impl ContextWidget for WidgetHitTestContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl WidgetHitTestContext<'_> {
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

impl Deref for WidgetHitTestContext<'_> {
    type Target = HitTestResult;

    fn deref(&self) -> &Self::Target {
        self.result
    }
}

impl DerefMut for WidgetHitTestContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.result
    }
}

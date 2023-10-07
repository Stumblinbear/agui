use std::ops::{Deref, DerefMut};

use glam::Mat4;

use crate::{
    element::{ContextElement, Element, ElementHitTestContext, ElementId},
    unit::{HitTest, HitTestResult, Offset},
    util::tree::Tree,
    widget::element::WidgetHitTestContext,
};

pub struct HitTestContext<'ctx> {
    pub(crate) widget_ctx: &'ctx mut WidgetHitTestContext<'ctx>,
}

impl ContextElement for HitTestContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.widget_ctx.get_elements()
    }

    fn get_element_id(&self) -> ElementId {
        self.widget_ctx.get_element_id()
    }
}

impl<'ctx> Deref for HitTestContext<'ctx> {
    type Target = WidgetHitTestContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        self.widget_ctx
    }
}

impl DerefMut for HitTestContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.widget_ctx
    }
}

pub struct IterChildrenHitTest<'ctx> {
    front_index: usize,
    back_index: usize,

    element_tree: &'ctx Tree<ElementId, Element>,

    children: &'ctx [ElementId],

    result: &'ctx mut HitTestResult,
}

impl<'ctx> IterChildrenHitTest<'ctx> {
    pub fn new(
        element_tree: &'ctx Tree<ElementId, Element>,
        children: &'ctx [ElementId],
        result: &'ctx mut HitTestResult,
    ) -> Self {
        IterChildrenHitTest {
            front_index: 0,
            back_index: children.len(),

            element_tree,

            children,

            result,
        }
    }
}

// TODO: refactor to LendingIterator when possible
impl IterChildrenHitTest<'_> {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<ChildElementHitTest> {
        if self.front_index >= self.back_index {
            return None;
        }

        self.front_index += 1;

        Some(ChildElementHitTest {
            element_tree: self.element_tree,

            result: self.result,

            index: self.front_index - 1,

            children: self.children,
        })
    }

    pub fn next_back(&mut self) -> Option<ChildElementHitTest> {
        if self.front_index >= self.back_index {
            return None;
        }

        self.back_index -= 1;

        Some(ChildElementHitTest {
            element_tree: self.element_tree,

            index: self.back_index,

            children: self.children,

            result: self.result,
        })
    }
}

pub struct ChildElementHitTest<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    index: usize,

    children: &'ctx [ElementId],

    result: &'ctx mut HitTestResult,
}

impl ChildElementHitTest<'_> {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn get_element_id(&self) -> ElementId {
        self.children[self.index]
    }

    pub fn get_element(&self) -> &Element {
        let element_id = self.get_element_id();

        self.element_tree
            .get(element_id)
            .expect("child element missing during hit test")
    }

    pub fn get_offset(&self) -> Offset {
        let element_id = self.get_element_id();

        self.element_tree
            .get(element_id)
            .expect("child element missing during hit test")
            .get_offset()
    }

    /// Check if the given position "hits" this widget or any of its descendants.
    ///
    /// The given position must be in the widget's local coordinate space, not the global
    /// coordinate space.
    pub fn hit_test(&mut self, position: Offset) -> HitTest {
        let element_id = self.get_element_id();

        let element = self
            .element_tree
            .get(element_id)
            .expect("child element missing during hit test");

        element.hit_test(
            ElementHitTestContext {
                element_tree: self.element_tree,

                element_id,

                result: self.result,
            },
            position,
        )
    }

    pub fn with_transform(
        &mut self,
        transform: Mat4,
        position: Offset,
        func: impl FnOnce(&mut ChildElementHitTest<'_>, Offset) -> HitTest,
    ) -> HitTest {
        self.result.push_transform(transform);

        let transformed_position = transform.project_point3(position.into());
        let transformed_position = Offset::new(transformed_position.x, transformed_position.y);

        let hit = func(self, transformed_position);

        self.result.pop_transform();

        hit
    }

    pub fn hit_test_with_transform(&mut self, transform: Mat4, position: Offset) -> HitTest {
        self.with_transform(transform, position, |child, position| {
            child.hit_test(position)
        })
    }

    pub fn with_offset(
        &mut self,
        offset: Offset,
        position: Offset,
        func: impl FnOnce(&mut ChildElementHitTest<'_>, Offset) -> HitTest,
    ) -> HitTest {
        self.result.push_offset(offset);

        let child_offset = self.get_offset();

        let transformed_position = position - child_offset;

        let hit = func(self, transformed_position);

        self.result.pop_transform();

        hit
    }

    pub fn hit_test_with_offset(&mut self, offset: Offset, position: Offset) -> HitTest {
        self.with_offset(offset, position, |child, position| child.hit_test(position))
    }
}

use crate::{
    element::{context::ElementHitTestContext, Element, ElementId},
    gestures::hit_test::HitTestEntry,
    unit::{Offset, Size},
    util::tree::Tree,
    widget::ContextWidget,
};

pub struct HitTestContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) path: &'ctx mut Vec<HitTestEntry>,

    pub(crate) element_id: ElementId,
    pub(crate) children: &'ctx [ElementId],
    pub(crate) size: &'ctx Size,
}

impl ContextWidget for HitTestContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl HitTestContext<'_> {
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    pub fn iter_children(&mut self) -> IterChildrenHitTest {
        IterChildrenHitTest::new(self.element_tree, self.children, self.path)
    }

    pub fn add_result(&mut self, entry: HitTestEntry) {
        self.path.push(entry);
    }
}

pub struct IterChildrenHitTest<'ctx> {
    front_index: usize,
    back_index: usize,

    element_tree: &'ctx Tree<ElementId, Element>,

    path: &'ctx mut Vec<HitTestEntry>,

    children: &'ctx [ElementId],
}

impl<'ctx> IterChildrenHitTest<'ctx> {
    pub fn new(
        element_tree: &'ctx Tree<ElementId, Element>,
        children: &'ctx [ElementId],
        path: &'ctx mut Vec<HitTestEntry>,
    ) -> Self {
        IterChildrenHitTest {
            front_index: 0,
            back_index: children.len(),

            element_tree,

            children,

            path,
        }
    }
}

// TODO: refactor to LendingIterator when possible
impl IterChildrenHitTest<'_> {
    pub fn next(&mut self) -> Option<ChildElementHitTest> {
        if self.front_index >= self.back_index {
            return None;
        }

        self.front_index += 1;

        Some(ChildElementHitTest {
            element_tree: self.element_tree,

            path: self.path,

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

            path: self.path,

            index: self.back_index,

            children: self.children,
        })
    }
}

pub struct ChildElementHitTest<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    index: usize,

    children: &'ctx [ElementId],

    path: &'ctx mut Vec<HitTestEntry>,
}

impl ChildElementHitTest<'_> {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn get_element_id(&self) -> ElementId {
        self.children[self.index]
    }

    pub fn hit_test(&mut self, position: Offset) -> bool {
        let element_id = self.get_element_id();

        let element = self
            .element_tree
            .get(element_id)
            .expect("child element missing during layout");

        element.hit_test(
            ElementHitTestContext {
                element_tree: self.element_tree,

                element_id,

                path: self.path,
            },
            position,
        )
    }
}

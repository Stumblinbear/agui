use crate::{
    element::{
        context::{ElementIntrinsicSizeContext, ElementLayoutContext},
        Element, ElementId,
    },
    unit::{Constraints, IntrinsicDimension, Offset, Size},
    util::tree::Tree,
};

pub struct IterChildren<'ctx> {
    index: usize,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) children: &'ctx [ElementId],
}

impl<'ctx> IterChildren<'ctx> {
    pub fn new(element_tree: &'ctx Tree<ElementId, Element>, children: &'ctx [ElementId]) -> Self {
        IterChildren {
            index: 0,

            element_tree,

            children,
        }
    }
}

// TODO: refactor to LendingIterator once it's available?
impl IterChildren<'_> {
    pub fn next(&mut self) -> Option<ChildElement> {
        if self.index >= self.children.len() {
            return None;
        }

        self.index += 1;

        Some(ChildElement {
            element_tree: self.element_tree,

            index: self.index - 1,

            children: self.children,
        })
    }
}

#[derive(Debug)]
pub struct ChildElement<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    index: usize,

    children: &'ctx [ElementId],
}

impl ChildElement<'_> {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn get_element_id(&self) -> ElementId {
        self.children[self.index]
    }

    pub fn compute_intrinsic_size(&self, dimension: IntrinsicDimension, cross_extent: f32) -> f32 {
        let element_id = self.get_element_id();

        let element = self
            .element_tree
            .get(element_id)
            .expect("child element missing during layout");

        element.intrinsic_size(
            ElementIntrinsicSizeContext {
                element_tree: self.element_tree,

                element_id,
            },
            dimension,
            cross_extent,
        )
    }
}

pub struct IterChildrenMut<'ctx> {
    index: usize,

    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) children: &'ctx [ElementId],
    pub(crate) offsets: &'ctx mut [Offset],
}

impl<'ctx> IterChildrenMut<'ctx> {
    pub fn new(
        element_tree: &'ctx mut Tree<ElementId, Element>,
        children: &'ctx [ElementId],
        offsets: &'ctx mut [Offset],
    ) -> Self {
        IterChildrenMut {
            index: 0,

            element_tree,

            children,
            offsets,
        }
    }
}

impl IterChildrenMut<'_> {
    pub fn next(&mut self) -> Option<ChildElementMut> {
        if self.index >= self.children.len() {
            return None;
        }

        self.index += 1;

        Some(ChildElementMut {
            element_tree: self.element_tree,

            index: self.index - 1,

            children: self.children,
            offsets: self.offsets,
        })
    }
}

#[derive(Debug)]
pub struct ChildElementMut<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    index: usize,

    children: &'ctx [ElementId],
    offsets: &'ctx mut [Offset],
}

impl ChildElementMut<'_> {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn get_element_id(&self) -> ElementId {
        self.children[self.index]
    }

    pub fn compute_intrinsic_size(&self, dimension: IntrinsicDimension, cross_extent: f32) -> f32 {
        let element_id = self.get_element_id();

        let element = self
            .element_tree
            .get(element_id)
            .expect("child element missing during layout");

        element.intrinsic_size(
            ElementIntrinsicSizeContext {
                element_tree: self.element_tree,

                element_id,
            },
            dimension,
            cross_extent,
        )
    }

    pub fn compute_layout(&mut self, constraints: impl Into<Constraints>) -> Size {
        let constraints = constraints.into();

        let element_id = self.get_element_id();

        self.element_tree
            .with(element_id, |element_tree, element| {
                element.layout(
                    ElementLayoutContext {
                        element_tree,

                        element_id,
                    },
                    constraints,
                )
            })
            .expect("child element missing during layout")
    }

    pub fn set_offset(&mut self, offset: impl Into<Offset>) {
        self.offsets[self.index] = offset.into();
    }
}

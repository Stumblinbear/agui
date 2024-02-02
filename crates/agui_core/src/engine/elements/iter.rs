use crate::{
    element::{Element, ElementId},
    engine::elements::ElementTree,
    util::tree::storage::HopSlotMapStorage,
};

pub trait ElementTreeIterator: Iterator {
    fn tree(&self) -> &ElementTree;
}

pub struct Iter<'query> {
    tree: &'query ElementTree,
    iter: crate::util::tree::iter::Iter<'query, ElementId, Element, HopSlotMapStorage>,
}

impl<'query> Iter<'query> {
    pub(super) fn new(tree: &'query ElementTree) -> Self {
        Self {
            tree,
            iter: tree.as_ref().iter(),
        }
    }
}

impl<'query> Iterator for Iter<'query> {
    type Item = ElementEntry<'query>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(id, node)| ElementEntry {
            id,
            parent: node.parent(),
            children: node.children(),
            element: node.borrow(),
        })
    }
}

impl<'query> ElementTreeIterator for Iter<'query> {
    fn tree(&self) -> &ElementTree {
        self.tree
    }
}

pub struct ElementEntry<'a> {
    id: ElementId,

    parent: Option<&'a ElementId>,
    children: &'a [ElementId],

    element: &'a Element,
}

impl<'a> ElementEntry<'a> {
    pub fn id(&self) -> ElementId {
        self.id
    }

    pub fn parent(&self) -> Option<&ElementId> {
        self.parent
    }

    pub fn children(&self) -> &[ElementId] {
        self.children
    }

    pub fn element(&self) -> &'a Element {
        self.element
    }
}

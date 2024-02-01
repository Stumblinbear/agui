use crate::{
    element::{lifecycle::ElementLifecycle, Element, ElementId},
    engine::elements::ElementTree,
    unit::Key,
    util::tree::TreeNode,
    widget::AnyWidget,
};

pub mod by_element;
pub mod by_key;
pub mod by_widget;

use self::by_element::QueryByElement;
use self::by_key::QueryByKey;
use self::by_widget::QueryByWidget;

#[derive(Clone)]
pub struct ElementQuery<'query> {
    tree: &'query ElementTree,
}

impl<'query> ElementQuery<'query> {
    pub(crate) fn new(tree: &'query ElementTree) -> ElementQuery<'query> {
        ElementQuery { tree }
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = (ElementId, &TreeNode<ElementId, Element>)> {
        self.tree.iter_nodes()
    }

    pub fn iter(&self) -> impl Iterator<Item = (ElementId, &Element)> {
        self.tree.iter()
    }

    pub fn by_key(self, key: Key) -> QueryByKey<'query>
    where
        Self: Sized,
    {
        QueryByKey::new(self.tree, key)
    }

    pub fn by_widget<W>(self) -> QueryByWidget<'query, W>
    where
        W: AnyWidget,
    {
        QueryByWidget::<W>::new(self.tree)
    }

    pub fn by_element<E>(self) -> QueryByElement<'query, E>
    where
        E: ElementLifecycle,
    {
        QueryByElement::<E>::new(self.tree)
    }
}

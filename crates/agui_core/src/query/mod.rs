use slotmap::hop::Iter;

use crate::{
    element::{lifecycle::ElementLifecycle, Element, ElementId},
    engine::widgets::key_storage::WidgetKeyStorage,
    unit::Key,
    util::tree::{Tree, TreeNode},
    widget::AnyWidget,
};

pub mod by_element;
pub mod by_key;
pub mod by_widget;

use self::by_element::QueryByElement;
use self::by_key::QueryByKey;
use self::by_widget::QueryByWidget;

pub trait WithWidgetKeyStorage {
    fn get_key(&self, element_id: ElementId) -> Option<Key>;

    fn get_element_key(&self, key: Key) -> Option<ElementId>;
}

#[derive(Clone)]
pub struct WidgetQuery<'query> {
    iter: Iter<'query, ElementId, TreeNode<ElementId, Element>>,
    key_storage: &'query WidgetKeyStorage,
}

impl WithWidgetKeyStorage for WidgetQuery<'_> {
    fn get_key(&self, element_id: ElementId) -> Option<Key> {
        self.key_storage.get_key(element_id)
    }

    fn get_element_key(&self, key: Key) -> Option<ElementId> {
        self.key_storage.get_element(key)
    }
}

impl<'query> WidgetQuery<'query> {
    pub(crate) fn new(
        tree: &'query Tree<ElementId, Element>,
        key_storage: &'query WidgetKeyStorage,
    ) -> WidgetQuery<'query> {
        WidgetQuery {
            iter: tree.iter(),
            key_storage,
        }
    }
}

impl<'query> Iterator for WidgetQuery<'query> {
    type Item = (ElementId, &'query Element);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(element_id, node)| (element_id, node.as_ref()))
    }
}

impl<'query> WidgetQuery<'query> {
    pub fn by_key(self, key: Key) -> QueryByKey<Self>
    where
        Self: Sized,
    {
        QueryByKey::new(self, key)
    }
}

pub trait WidgetQueryExt<'query> {
    fn by_widget<W>(self) -> QueryByWidget<Self, W>
    where
        Self: Sized,
        W: AnyWidget;

    fn by_element<E>(self) -> QueryByElement<Self, E>
    where
        Self: Sized,
        E: ElementLifecycle;
}

impl<'query, I> WidgetQueryExt<'query> for I
where
    I: Iterator<Item = (ElementId, &'query Element)>,
{
    fn by_widget<W>(self) -> QueryByWidget<Self, W>
    where
        W: AnyWidget,
    {
        QueryByWidget::<Self, W>::new(self)
    }

    fn by_element<E>(self) -> QueryByElement<Self, E>
    where
        E: ElementLifecycle,
    {
        QueryByElement::<Self, E>::new(self)
    }
}

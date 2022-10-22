use slotmap::hop::Iter;

use crate::{
    manager::element::WidgetElement,
    unit::Key,
    util::tree::{Tree, TreeNode},
    widget::{WidgetId, WidgetState, WidgetView},
};

pub mod by_key;
pub mod by_type;

use self::{by_key::QueryByKey, by_type::QueryByType};

pub struct WidgetQuery<'query> {
    pub iter: Iter<'query, WidgetId, TreeNode<WidgetId, WidgetElement>>,
}

impl<'query> WidgetQuery<'query> {
    pub(crate) fn new(tree: &'query Tree<WidgetId, WidgetElement>) -> WidgetQuery<'query> {
        WidgetQuery { iter: tree.iter() }
    }
}

impl<'query> Iterator for WidgetQuery<'query> {
    type Item = &'query WidgetElement;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|(_, node)| node.value.as_ref())
    }
}

pub trait WidgetQueryExt<'query> {
    fn by_key(self, key: Key) -> QueryByKey<Self>
    where
        Self: Sized;

    fn by_type<W>(self) -> QueryByType<Self, W>
    where
        Self: Sized,
        W: WidgetView + WidgetState;
}

impl<'query, I> WidgetQueryExt<'query> for I
where
    I: Iterator<Item = &'query WidgetElement>,
{
    fn by_key(self, key: Key) -> QueryByKey<Self> {
        QueryByKey::new(self, key)
    }

    fn by_type<W>(self) -> QueryByType<Self, W>
    where
        W: WidgetView + WidgetState,
    {
        QueryByType::<Self, W>::new(self)
    }
}

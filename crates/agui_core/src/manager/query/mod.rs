use slotmap::hop::Iter;

use crate::{
    unit::Key,
    util::tree::TreeNode,
    widget::{BoxedWidget, WidgetImpl, WidgetId},
};

use super::WidgetManager;

pub mod by_key;
pub mod by_type;

use self::{by_key::QueryByKey, by_type::QueryByType};

pub struct WidgetQuery<'query> {
    pub iter: Iter<'query, WidgetId, TreeNode<WidgetId, BoxedWidget>>,
}

impl<'query> WidgetQuery<'query> {
    pub(in crate::manager) fn new(manager: &'query WidgetManager) -> WidgetQuery<'query> {
        WidgetQuery {
            iter: manager.widget_tree.iter(),
        }
    }
}

impl<'query> Iterator for WidgetQuery<'query> {
    type Item = &'query BoxedWidget;

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
        W: WidgetImpl;
}

impl<'query, I> WidgetQueryExt<'query> for I
where
    I: Iterator<Item = &'query BoxedWidget>,
{
    fn by_key(self, key: Key) -> QueryByKey<Self> {
        QueryByKey::new(self, key)
    }

    fn by_type<W>(self) -> QueryByType<Self, W>
    where
        W: WidgetImpl,
    {
        QueryByType::<Self, W>::new(self)
    }
}

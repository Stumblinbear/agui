use slotmap::hop::Iter;

use crate::{
    unit::Key,
    widget::{Widget, WidgetId},
};

pub mod by_key;
pub mod by_type;

use self::{by_key::QueryByKey, by_type::QueryByType};

use super::{tree::TreeNode, widget::WidgetBuilder, WidgetManager};

pub struct WidgetQuery<'query> {
    pub iter: Iter<'query, WidgetId, TreeNode<WidgetId, Widget>>,
}

impl<'query> WidgetQuery<'query> {
    pub(in crate::manager) fn new(manager: &'query WidgetManager) -> WidgetQuery<'query> {
        WidgetQuery {
            iter: manager.tree.iter(),
        }
    }
}

impl<'query> Iterator for WidgetQuery<'query> {
    type Item = &'query Widget;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|(_, node)| Some(&**node))
    }
}

pub trait WidgetQueryExt<'query> {
    fn by_key(self, key: Key) -> QueryByKey<Self>
    where
        Self: Sized;

    fn by_type<W>(self) -> QueryByType<Self, W>
    where
        Self: Sized,
        W: WidgetBuilder;
}

impl<'query, I> WidgetQueryExt<'query> for I
where
    I: Iterator<Item = &'query Widget>,
{
    fn by_key(self, key: Key) -> QueryByKey<Self> {
        QueryByKey::new(self, key)
    }

    fn by_type<W>(self) -> QueryByType<Self, W>
    where
        W: WidgetBuilder,
    {
        QueryByType::<Self, W>::new(self)
    }
}

use std::{
    any::TypeId,
    cell::{Ref, RefCell, RefMut},
    fmt::Debug,
    rc::Rc,
};

use slotmap::new_key_type;

use crate::{
    engine::{
        tree::Tree,
        widget::{WidgetBuilder, WidgetElement, WidgetImpl},
    },
    unit::Key,
};

mod context;
mod node;
mod result;

pub use self::{
    context::*,
    node::{StatefulWidget, StatelessWidget},
    result::BuildResult,
};

new_key_type! {
    pub struct WidgetId;
}

impl std::fmt::Display for WidgetId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<'ui> morphorm::Node<'ui> for WidgetId {
    type Data = Tree<Self, Widget>;

    fn layout_type(&self, store: &'_ Self::Data) -> Option<morphorm::LayoutType> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout_type())
            .map(Into::into)
    }

    fn position_type(&self, store: &'_ Self::Data) -> Option<morphorm::PositionType> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.position.into())
    }

    fn width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.sizing.get_width().into())
    }

    fn height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.sizing.get_height().into())
    }

    fn min_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.min_sizing.get_width().into())
    }

    fn min_height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.min_sizing.get_height().into())
    }

    fn max_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.max_sizing.get_width().into())
    }

    fn max_height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.max_sizing.get_height().into())
    }

    fn top(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .and_then(|layout| layout.position.get_top())
            .map(Into::into)
    }

    fn right(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .and_then(|layout| layout.position.get_right())
            .map(Into::into)
    }

    fn bottom(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .and_then(|layout| layout.position.get_bottom())
            .map(Into::into)
    }

    fn left(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .and_then(|layout| layout.position.get_left())
            .map(Into::into)
    }

    fn min_top(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn max_top(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn min_right(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn max_right(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn min_bottom(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn max_bottom(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn min_left(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn max_left(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn child_top(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.margin.get_top().into())
    }

    fn child_right(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.margin.get_right().into())
    }

    fn child_bottom(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.margin.get_bottom().into())
    }

    fn child_left(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.margin.get_left().into())
    }

    fn row_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout_type())
            .and_then(|layout_type| layout_type.get_column_spacing())
            .map(Into::into)
    }

    fn col_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout_type())
            .and_then(|layout_type| layout_type.get_row_spacing())
            .map(Into::into)
    }

    fn grid_rows(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout_type())
            .and_then(|layout_type| layout_type.get_rows())
            .map(|val| val.into_iter().map(Into::into).collect())
    }

    fn grid_cols(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout_type())
            .and_then(|layout_type| layout_type.get_columns())
            .map(|val| val.into_iter().map(Into::into).collect())
    }

    fn row_index(&self, _store: &'_ Self::Data) -> Option<usize> {
        Some(0)
    }

    fn col_index(&self, _store: &'_ Self::Data) -> Option<usize> {
        Some(0)
    }

    fn row_span(&self, _store: &'_ Self::Data) -> Option<usize> {
        Some(1)
    }

    fn col_span(&self, _store: &'_ Self::Data) -> Option<usize> {
        Some(1)
    }

    fn border_top(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn border_right(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn border_bottom(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn border_left(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetKey(Option<WidgetId>, Key);

impl WidgetKey {
    pub fn get_owner(&self) -> Option<WidgetId> {
        self.0
    }

    pub fn get_key(&self) -> Key {
        self.1
    }
}

// #[derive(Default, Clone)]
// pub struct ChildWidget(Option<Widget>);

// impl Deref for ChildWidget {
//     type Target = Option<Widget>;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl std::fmt::Debug for ChildWidget {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match &self.0 {
//             Some(widget) => widget.fmt(f),
//             None => write!(f, "None"),
//         }
//     }
// }

#[derive(Clone)]
pub enum Widget {
    None,

    Some {
        key: Option<WidgetKey>,
        inner: Rc<RefCell<dyn WidgetImpl>>,
    },
}

impl Default for Widget {
    fn default() -> Self {
        Self::None
    }
}

impl Widget {
    pub(crate) fn new<W>(key: Option<WidgetKey>, widget: W) -> Self
    where
        W: WidgetImpl,
    {
        Self::Some {
            key,
            inner: Rc::new(RefCell::new(widget)),
        }
    }

    pub fn get_key(&self) -> Option<WidgetKey> {
        if let Widget::Some { key, .. } = self {
            return *key;
        }

        None
    }

    pub fn get(&self) -> Option<Ref<dyn WidgetImpl>> {
        match self {
            Widget::None => None,
            Widget::Some { inner, .. } => Some(inner.borrow()),
        }
    }

    pub fn get_mut(&self) -> Option<RefMut<dyn WidgetImpl>> {
        match self {
            Widget::None => None,
            Widget::Some { inner, .. } => Some(inner.borrow_mut()),
        }
    }

    pub fn get_as<W>(&self) -> Option<Ref<WidgetElement<W>>>
    where
        W: WidgetBuilder,
    {
        if let Widget::Some { inner, .. } = self {
            let widget = RefCell::borrow(inner);

            if widget.get_type_id() == TypeId::of::<W>() {
                return Some(Ref::map(widget, |x| x.downcast_ref().unwrap()));
            }
        }

        None
    }

    pub fn get_as_mut<W>(&self) -> Option<RefMut<WidgetElement<W>>>
    where
        W: WidgetBuilder,
    {
        if let Widget::Some { inner, .. } = self {
            let widget = RefCell::borrow_mut(inner);

            if widget.get_type_id() == TypeId::of::<W>() {
                return Some(RefMut::map(widget, |x| x.downcast_mut().unwrap()));
            }
        }

        None
    }
}

impl std::fmt::Debug for Widget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Some { key, inner } => f
                .debug_struct("Widget")
                .field("key", &key)
                .field("inner", &inner.borrow())
                .finish(),
        }
    }
}

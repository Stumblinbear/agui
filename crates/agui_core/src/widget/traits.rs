use std::{any::Any, rc::Rc};

use crate::{
    element::{Element, ElementBuilder},
    unit::{AsAny, Key},
};

use super::Widget;

pub trait AnyWidget: AsAny {
    fn as_any(self: Rc<Self>) -> Rc<dyn Any>;

    fn widget_name(&self) -> &'static str;

    fn create_element(self: Rc<Self>, key: Option<Key>) -> Element;
}

impl<T> AnyWidget for T
where
    T: ElementBuilder + 'static,
{
    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn widget_name(&self) -> &'static str {
        let type_name = std::any::type_name::<T>();

        type_name
            .split('<')
            .next()
            .unwrap_or(type_name)
            .split("::")
            .last()
            .unwrap_or(type_name)
    }

    fn create_element(self: Rc<Self>, key: Option<Key>) -> Element {
        Element::new(
            self.widget_name(),
            key,
            ElementBuilder::create_element(self),
        )
    }
}

pub trait IntoWidget {
    fn into_widget(self) -> Widget;
}

impl<W> From<W> for Widget
where
    W: IntoWidget + ElementBuilder + 'static,
{
    fn from(widget: W) -> Self {
        Widget::new(widget)
    }
}

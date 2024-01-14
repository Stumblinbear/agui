use std::{any::Any, rc::Rc};

use crate::element::ElementBuilder;

use super::Widget;

pub trait AnyWidget: ElementBuilder {
    fn as_any(self: Rc<Self>) -> Rc<dyn Any>;

    fn widget_type_id(&self) -> std::any::TypeId;

    fn widget_name(&self) -> &'static str;
}

impl<T> AnyWidget for T
where
    T: ElementBuilder,
{
    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn widget_type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<T>()
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
}

pub trait IntoWidget {
    fn into_widget(self) -> Widget;
}

impl<W> From<W> for Widget
where
    W: IntoWidget + ElementBuilder,
{
    fn from(widget: W) -> Self {
        Widget::new(widget)
    }
}

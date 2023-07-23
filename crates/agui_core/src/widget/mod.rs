use std::{any::Any, rc::Rc};

mod context;
pub mod element;
mod inherited;
mod key;
mod layout;
mod paint;
mod r#ref;
mod stateful;
mod stateless;

pub use self::{
    context::*, inherited::*, key::*, layout::*, paint::*, r#ref::*, stateful::*, stateless::*,
};

pub trait ElementBuilder: 'static {
    fn create_element(self: Rc<Self>) -> Box<dyn self::element::WidgetElement>;
}

pub trait AnyWidget: ElementBuilder {
    fn as_any(self: Rc<Self>) -> Rc<dyn Any>;

    fn widget_name(&self) -> &'static str;
}

impl<T> AnyWidget for T
where
    T: ElementBuilder,
{
    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn widget_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl dyn AnyWidget {
    pub fn downcast<T: AnyWidget>(self: Rc<Self>) -> Option<Rc<T>> {
        AnyWidget::as_any(self).downcast().ok()
    }
}

pub trait IntoWidget: ElementBuilder {
    fn into_widget(self) -> WidgetRef;
}

impl<W> IntoWidget for W
where
    W: ElementBuilder,
{
    fn into_widget(self) -> WidgetRef {
        WidgetRef::new(self)
    }
}

impl<W> From<W> for WidgetRef
where
    W: IntoWidget,
{
    fn from(widget: W) -> Self {
        WidgetRef::new(widget)
    }
}

pub trait IntoChildren {
    fn into_children(self) -> Vec<WidgetRef>;
}

impl IntoChildren for () {
    fn into_children(self) -> Vec<WidgetRef> {
        Vec::new()
    }
}

impl IntoChildren for Vec<WidgetRef> {
    fn into_children(self) -> Vec<WidgetRef> {
        self
    }
}

impl IntoChildren for WidgetRef {
    fn into_children(self) -> Vec<WidgetRef> {
        vec![self]
    }
}

impl<T: IntoWidget> IntoChildren for T {
    fn into_children(self) -> Vec<WidgetRef> {
        vec![self.into_widget()]
    }
}

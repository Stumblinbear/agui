use std::{any::Any, rc::Rc};

mod context;
pub mod inheritance;
mod inherited;
pub mod instance;
pub mod key;
mod r#ref;
mod state;
mod view;

use self::{instance::ElementWidget, key::WidgetKey};

pub use self::{context::*, inherited::*, r#ref::*, state::*, view::*};

pub trait WidgetBuilder: 'static {
    fn create_element(self: Rc<Self>) -> Box<dyn ElementWidget>;
}

pub trait AnyWidget: WidgetBuilder {
    fn as_any(self: Rc<Self>) -> Rc<dyn Any>;

    fn type_name(&self) -> &'static str;
}

impl<T> AnyWidget for T
where
    T: WidgetBuilder,
{
    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl dyn AnyWidget {
    pub fn downcast<T: AnyWidget>(self: Rc<Self>) -> Option<Rc<T>> {
        AnyWidget::as_any(self).downcast().ok()
    }
}

pub trait IntoWidget: WidgetBuilder {
    fn into_widget(self) -> WidgetRef;
}

impl<W> IntoWidget for W
where
    W: WidgetBuilder,
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

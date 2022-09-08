use std::rc::Rc;

use slotmap::new_key_type;

mod context;
mod element;
mod instance;
mod key;
mod result;
mod widget_impl;

pub use self::{context::*, element::*, instance::*, key::*, result::*, widget_impl::*};

new_key_type! {
    pub struct WidgetId;
}

pub type BoxedWidget = Box<dyn WidgetInstance>;

pub trait IntoWidget {
    fn into_widget(self: Rc<Self>) -> BoxedWidget;
}

#[derive(Default, Clone)]
pub struct Widget {
    key: Option<WidgetKey>,

    inner: Option<Rc<dyn IntoWidget>>,
}

impl Widget {
    pub fn new<W>(widget: W) -> Self
    where
        W: IntoWidget + 'static,
    {
        Widget {
            key: None,
            inner: Some(Rc::new(widget)),
        }
    }

    pub(crate) fn create(self) -> Option<BoxedWidget> {
        self.inner.map(|inner| {
            let mut widget = inner.into_widget();

            if let Some(key) = self.key {
                widget.set_key(key);
            }

            widget
        })
    }

    pub(crate) fn set_key(&mut self, key: WidgetKey) {
        self.key = Some(key);
    }

    pub(crate) fn get_key(&self) -> &Option<WidgetKey> {
        &self.key
    }
}

impl<W> From<W> for Widget
where
    W: IntoWidget + 'static,
{
    fn from(widget: W) -> Self {
        Widget::new(widget)
    }
}

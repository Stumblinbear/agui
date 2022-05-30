use std::rc::Rc;

use slotmap::new_key_type;

use super::{WidgetInstance, WidgetKey};

new_key_type! {
    pub struct WidgetId;
}

pub type BoxedWidget = Box<dyn WidgetInstance>;

pub trait IntoWidget: std::fmt::Debug + 'static {
    fn into_widget(self: Rc<Self>) -> BoxedWidget;
}

#[derive(Debug, Default, Clone)]
pub struct Widget {
    key: Option<WidgetKey>,

    inner: Option<Rc<dyn IntoWidget>>,
}

impl Widget {
    pub fn new<W>(widget: W) -> Self
    where
        W: IntoWidget,
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

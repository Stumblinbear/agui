use std::{hash::Hash, rc::Rc};

use slotmap::new_key_type;

mod context;
mod descriptor;
mod element;
mod instance;
mod key;
mod result;
mod widget_impl;

pub use self::{
    context::*, descriptor::*, element::*, instance::*, key::*, result::*, widget_impl::*,
};

new_key_type! {
    pub struct WidgetId;
}

pub type BoxedWidget = Box<dyn WidgetInstance>;

pub trait IntoWidget {
    fn into_widget(self: Rc<Self>, desc: WidgetDescriptor) -> BoxedWidget;
}

#[derive(Default, Clone)]
pub struct Widget {
    desc: Option<WidgetDescriptor>,
}

impl Widget {
    pub(crate) fn get_descriptor(&self) -> &Option<WidgetDescriptor> {
        &self.desc
    }

    pub(crate) fn set_key(&mut self, key: WidgetKey) {
        if let Some(desc) = self.desc.as_mut() {
            if desc.key.is_some() {
                tracing::warn!(
                    key = format!("{:?}", key).as_str(),
                    "cannot key a widget that has already been keyed, ignoring"
                );
            } else {
                desc.key = Some(key);
            }
        }
    }
}

impl<W> From<W> for Widget
where
    W: IntoWidget + 'static,
{
    fn from(widget: W) -> Self {
        Widget {
            desc: Some(WidgetDescriptor {
                key: None,
                inner: Rc::new(widget),
            }),
        }
    }
}

impl From<&Widget> for Widget {
    fn from(widget: &Widget) -> Self {
        widget.to_owned()
    }
}

use std::{any::Any, rc::Rc};

mod children;
mod context;
mod inherited;
pub mod instance;
pub mod key;
mod state;
mod view;

use crate::element::Element;

use self::{instance::ElementWidget, key::WidgetKey};

pub use self::{children::*, context::*, inherited::*, state::*, view::*};

pub trait AnyWidget: 'static {
    fn as_any(self: Rc<Self>) -> Rc<dyn Any>;

    fn type_name(&self) -> &'static str;
}

impl<T> AnyWidget for T
where
    T: WidgetView + 'static,
{
    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

pub trait IntoElementWidget: AnyWidget {
    fn into_element_widget(self: Rc<Self>) -> Box<dyn ElementWidget>;
}

#[derive(Default, Clone)]
pub enum WidgetRef {
    #[default]
    None,
    Some(Option<WidgetKey>, Rc<dyn IntoElementWidget>),
}

impl WidgetRef {
    pub fn new<W>(widget: W) -> Self
    where
        W: IntoElementWidget,
    {
        Self::new_with_key(None, widget)
    }

    pub fn new_with_key<W>(key: Option<WidgetKey>, widget: W) -> Self
    where
        W: IntoElementWidget,
    {
        Self::Some(key, Rc::new(widget))
    }

    pub fn get_display_name(&self) -> Option<&str> {
        if let Self::Some(.., widget) = self {
            let type_name = widget.type_name();

            Some(
                type_name
                    .split('<')
                    .next()
                    .unwrap_or(type_name)
                    .split("::")
                    .last()
                    .unwrap_or(type_name),
            )
        } else {
            None
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    pub fn get_key(&self) -> Option<&WidgetKey> {
        if let Self::Some(key, ..) = self {
            key.as_ref()
        } else {
            None
        }
    }

    pub fn downcast<W>(&self) -> Option<Rc<W>>
    where
        W: WidgetView,
    {
        if let Self::Some(.., widget) = self {
            Rc::clone(widget).as_any().downcast::<W>().ok()
        } else {
            None
        }
    }

    pub(crate) fn create(&self) -> Option<Element> {
        if let Self::Some(key, widget) = self {
            Some(Element::new(*key, Rc::clone(widget).into_element_widget()))
        } else {
            None
        }
    }
}

impl std::fmt::Debug for WidgetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Self::Some(key, ..) = self {
            f.write_str("WidgetRef::Some(")?;

            f.write_str(self.get_display_name().unwrap())?;

            if let Some(key) = key {
                f.write_str(" <key: ")?;
                key.fmt(f)?;
                f.write_str(">")?;
            }

            Ok(())
        } else {
            f.debug_struct("WidgetRef::None").finish()
        }
    }
}

impl std::fmt::Display for WidgetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Self::Some(key, ..) = self {
            f.write_str(self.get_display_name().unwrap())?;

            if let Some(key) = key {
                f.write_str(" <key: ")?;
                key.fmt(f)?;
                f.write_str(">")?;
            }

            Ok(())
        } else {
            f.debug_struct("None").finish()
        }
    }
}

impl From<&WidgetRef> for WidgetRef {
    fn from(widget: &WidgetRef) -> Self {
        widget.to_owned()
    }
}

pub trait IntoWidget: IntoElementWidget {
    fn into_widget(self) -> WidgetRef;
}

impl<W> IntoWidget for W
where
    W: IntoElementWidget,
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

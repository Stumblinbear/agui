use std::rc::Rc;

use crate::element::Element;

use super::{key::WidgetKey, AnyWidget};

#[derive(Default, Clone)]
pub enum WidgetRef {
    #[default]
    None,
    Some(Option<WidgetKey>, Rc<dyn AnyWidget>),
}

impl WidgetRef {
    pub fn new<W>(widget: W) -> Self
    where
        W: AnyWidget,
    {
        Self::new_with_key(None, widget)
    }

    pub fn new_with_key<W>(key: Option<WidgetKey>, widget: W) -> Self
    where
        W: AnyWidget,
    {
        Self::Some(key, Rc::new(widget))
    }

    pub fn get_display_name(&self) -> Option<&str> {
        if let Self::Some(.., widget) = self {
            let type_name = widget.widget_name();

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
        W: AnyWidget,
    {
        if let Self::Some(.., widget) = self {
            Rc::clone(widget).as_any().downcast::<W>().ok()
        } else {
            None
        }
    }

    pub fn is<W>(&self) -> bool
    where
        W: AnyWidget,
    {
        if let Self::Some(.., widget) = self {
            Rc::clone(widget).as_any().is::<W>()
        } else {
            false
        }
    }

    pub(crate) fn create(&self) -> Option<Element> {
        if let Self::Some(key, widget) = self {
            Some(Element::new(*key, Rc::clone(widget).create_element()))
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

            f.write_str(")")?;

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

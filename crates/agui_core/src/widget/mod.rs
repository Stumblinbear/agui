use std::rc::Rc;

use downcast_rs::{impl_downcast, Downcast};

mod builder;
mod context;
mod inherited;
pub mod instance;
pub mod key;
mod result;
mod state;

use crate::element::ElementType;

use self::key::WidgetKey;

pub use self::{builder::*, context::*, inherited::*, result::*, state::*};

pub trait WidgetDerive: Downcast {
    fn get_type_name(&self) -> &str;

    fn is_equal(&self, other: &dyn WidgetDerive) -> bool;

    fn create_element(self: Rc<Self>) -> ElementType;
}

impl_downcast!(WidgetDerive);

pub trait Widget: WidgetDerive + WidgetState + PartialEq {}

#[derive(Default, Clone)]
pub enum WidgetRef {
    #[default]
    None,
    Some {
        key: Option<WidgetKey>,
        widget: Rc<dyn WidgetDerive>,
    },
}

impl WidgetRef {
    pub fn new<W>(widget: W) -> Self
    where
        W: Widget,
    {
        Self::new_with_key(None, widget)
    }

    pub fn new_with_key<W>(key: Option<WidgetKey>, widget: W) -> Self
    where
        W: Widget,
    {
        Self::Some {
            key,

            widget: Rc::new(widget),
        }
    }

    pub fn get_display_name(&self) -> Option<String> {
        if let Self::Some { widget, .. } = self {
            let type_name = widget.get_type_name();

            let display_name = if !type_name.contains('<') {
                String::from(type_name.rsplit("::").next().unwrap())
            } else {
                let mut name = String::new();

                let mut remaining = String::from(type_name);

                while let Some((part, rest)) = remaining.split_once('<') {
                    name.push_str(part.rsplit("::").next().unwrap());

                    name.push('<');

                    remaining = String::from(rest);
                }

                name.push_str(remaining.rsplit("::").next().unwrap());

                name
            };

            Some(display_name)
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
        if let Self::Some { key, .. } = self {
            key.as_ref()
        } else {
            None
        }
    }

    pub fn downcast_rc<W>(&self) -> Option<Rc<W>>
    where
        W: Widget,
    {
        if let Self::Some { widget, .. } = self {
            Rc::clone(widget).downcast_rc::<W>().ok()
        } else {
            None
        }
    }

    pub(crate) fn create(&self) -> Option<ElementType> {
        if let Self::Some { widget, .. } = self {
            Some(Rc::clone(widget).create_element())
        } else {
            None
        }
    }
}

impl PartialEq for WidgetRef {
    fn eq(&self, other: &Self) -> bool {
        if let Self::Some { key, widget } = self {
            if let Self::Some {
                key: other_key,
                widget: other_widget,
            } = other
            {
                if key.is_some() || other_key.is_some() {
                    // If either one has a key, this will always override equality checks
                    return key == other_key;
                }

                return widget.as_ref().is_equal(other_widget.as_ref());
            }
        }

        false
    }
}

impl Eq for WidgetRef {}

// impl Hash for WidgetRef {
//     fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//         self.get_type_id().hash(state);

//         if let Self::Some { key, widget, .. } = self {
//             if let Some(key) = key {
//                 // The key is effectively the hash of this reference
//                 key.hash(state);
//             } else {
//                 Rc::as_ptr(widget).hash(state);
//             }
//         }
//     }
// }

impl std::fmt::Debug for WidgetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Self::Some { key, .. } = self {
            f.write_str("WidgetRef::Some(")?;

            f.write_str(&self.get_display_name().unwrap())?;

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
        if let Self::Some { key, .. } = self {
            f.write_str(&self.get_display_name().unwrap())?;

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

impl<W> From<W> for WidgetRef
where
    W: Widget,
{
    fn from(widget: W) -> Self {
        WidgetRef::new(widget)
    }
}

impl From<&WidgetRef> for WidgetRef {
    fn from(widget: &WidgetRef) -> Self {
        widget.to_owned()
    }
}

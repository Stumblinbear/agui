use std::{
    any::{type_name, TypeId},
    cell::RefCell,
    hash::Hash,
    rc::Rc,
};

use slotmap::{new_key_type, Key};

mod builder;
mod context;
pub mod dispatch;
pub mod instance;
pub mod key;
mod result;

use self::{dispatch::WidgetDispatch, key::WidgetKey};

pub use self::{builder::*, context::*, result::*};

new_key_type! {
    pub struct WidgetId;
}

#[derive(Default, Clone)]
pub enum WidgetRef {
    #[default]
    None,
    Some {
        key: Option<WidgetKey>,

        display_name: String,
        widget: Rc<dyn Widget>,

        widget_id: Rc<RefCell<WidgetId>>,
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
        let type_name = type_name::<W>();

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

        Self::Some {
            key,

            display_name,
            widget: Rc::new(widget),

            widget_id: Rc::default(),
        }
    }

    pub fn get_type_id(&self) -> Option<TypeId> {
        if let Self::Some { widget, .. } = self {
            Some(widget.type_id())
        } else {
            None
        }
    }

    pub fn get_display_name(&self) -> Option<&str> {
        if let Self::Some { display_name, .. } = self {
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

    pub(crate) fn get_current_id(&self) -> WidgetId {
        if let Self::Some { widget_id, .. } = self {
            *widget_id.as_ref().borrow()
        } else {
            WidgetId::default()
        }
    }

    pub(crate) fn set_current_id(&self, current_widget_id: WidgetId) {
        if let Self::Some { widget_id, .. } = self {
            *widget_id.as_ref().borrow_mut() = current_widget_id;
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

    pub(crate) fn create(&self) -> Option<Box<dyn WidgetDispatch>> {
        if let Self::Some { widget, .. } = self {
            Some(Rc::clone(widget).into_widget())
        } else {
            None
        }
    }
}

impl PartialEq for WidgetRef {
    fn eq(&self, other: &Self) -> bool {
        if self.get_type_id() != other.get_type_id() {
            return false;
        }

        if let Self::Some { key, widget_id, .. } = self {
            if let Self::Some {
                key: other_key,
                widget_id: other_widget_id,
                ..
            } = other
            {
                if key.is_some() || other_key.is_some() {
                    // If either one has a key, this will always override equality checks
                    return key == other_key;
                }

                // If either of them are null, one of them isn't in the tree and are not equal
                if widget_id.borrow().is_null() || other_widget_id.borrow().is_null() {
                    return false;
                }

                // If the two widget_ids are equal, then the two widgets are equal and currently exist in the tree
                return widget_id == other_widget_id;
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
        if let Self::Some {
            key, display_name, ..
        } = self
        {
            f.write_str("WidgetRef::Some(")?;

            f.write_str(display_name)?;

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
        if let Self::Some {
            key, display_name, ..
        } = self
        {
            f.write_str(display_name)?;

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

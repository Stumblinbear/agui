use std::{hash::Hash, rc::Rc};

use super::{BoxedWidget, IntoWidget, WidgetKey};

#[derive(Clone)]
pub struct WidgetDescriptor {
    pub(super) key: Option<WidgetKey>,

    pub(super) inner: Rc<dyn IntoWidget>,
}

impl WidgetDescriptor {
    pub fn get_key(&self) -> Option<&WidgetKey> {
        self.key.as_ref()
    }

    pub(crate) fn create(&self) -> BoxedWidget {
        Rc::clone(&self.inner).into_widget(self.clone())
    }
}

impl std::fmt::Debug for WidgetDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Widget")
            .field("key", &self.key)
            .field("ref", &Rc::into_raw(self.inner.clone()))
            .finish()
    }
}

impl Eq for WidgetDescriptor {}

impl PartialEq for WidgetDescriptor {
    fn eq(&self, other: &Self) -> bool {
        if self.key.is_some() || other.key.is_some() {
            return self.key == other.key;
        }

        return Rc::ptr_eq(&self.inner, &other.inner);
    }
}

impl Hash for WidgetDescriptor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if let Some(key) = self.key {
            key.hash(state);
        } else {
            Rc::as_ptr(&self.inner).hash(state);
        }
    }
}

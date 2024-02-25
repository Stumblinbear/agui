use std::{
    cell::RefCell,
    hash::{Hash, Hasher},
    rc::Rc,
};

use rustc_hash::FxHashMap;

use crate::{element::Element, reactivity::strategies::WithReactiveKey, unit::Key};

mod traits;

pub use traits::*;

#[derive(Clone)]
pub struct Widget {
    key: Option<Key>,
    widget: Rc<dyn AnyWidget>,
}

impl Widget {
    pub fn new<W>(widget: W) -> Self
    where
        W: AnyWidget,
    {
        Self {
            key: None,
            widget: Rc::new(widget),
        }
    }

    pub fn new_with_key<W>(key: Key, widget: W) -> Self
    where
        W: AnyWidget,
    {
        Self {
            key: Some(key),
            widget: Rc::new(widget),
        }
    }

    pub fn widget_name(&self) -> &'static str {
        (*self.widget).widget_name()
    }

    pub fn key(&self) -> Option<Key> {
        self.key
    }

    pub fn downcast<W>(&self) -> Option<Rc<W>>
    where
        W: AnyWidget,
    {
        Rc::clone(&self.widget).as_any().downcast::<W>().ok()
    }

    pub fn create_element(self) -> Element {
        self.widget.create_element()
    }
}

impl WithReactiveKey for Widget {
    fn key(&self) -> Option<Key> {
        self.key
    }
}

impl PartialEq for Widget {
    fn eq(&self, other: &Self) -> bool {
        if self.key.is_some() || other.key.is_some() {
            return self.key == other.key;
        }

        Rc::ptr_eq(&self.widget, &other.widget)
    }
}

impl<T> PartialEq<Rc<T>> for Widget
where
    T: AnyWidget,
{
    fn eq(&self, other: &Rc<T>) -> bool {
        std::ptr::eq(
            Rc::as_ptr(&self.widget) as *const _ as *const (),
            Rc::as_ptr(other) as *const _ as *const (),
        )
    }
}

impl Eq for Widget {}

impl Hash for Widget {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(key) = self.key {
            key.hash(state);

            return;
        }

        std::ptr::hash(Rc::as_ptr(&self.widget) as *const _ as *const (), state);
    }
}

impl std::fmt::Debug for Widget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.widget_name())?;

        f.write_str("#")?;

        f.write_str(&format!(
            "{:p}",
            Rc::as_ptr(&self.widget) as *const _ as *const ()
        ))?;

        if let Some(key) = self.key {
            f.write_str(" <key: ")?;
            key.fmt(f)?;
            f.write_str(">")?;
        }

        Ok(())
    }
}

impl std::fmt::Display for Widget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.widget_name())?;

        if let Some(key) = self.key {
            f.write_str(" <key: ")?;
            key.fmt(f)?;
            f.write_str(">")?;
        }

        Ok(())
    }
}

impl IntoWidget for Widget {
    fn into_widget(self) -> Widget {
        self
    }
}

impl IntoWidget for &Widget {
    fn into_widget(self) -> Widget {
        self.clone()
    }
}

type WidgetFn = fn() -> Widget;

thread_local! {
    static FN_WIDGETS: RefCell<FxHashMap<WidgetFn, Widget>> = RefCell::default();
}

impl IntoWidget for WidgetFn {
    fn into_widget(self) -> Widget {
        FN_WIDGETS.with(|widgets| {
            let mut widgets = widgets.borrow_mut();

            widgets.entry(self).or_insert_with(self).clone()
        })
    }
}

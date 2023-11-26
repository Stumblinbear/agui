use std::{
    cell::RefCell,
    hash::{Hash, Hasher},
    rc::Rc,
};

use rustc_hash::FxHashMap;

use crate::{element::ElementType, unit::Key, util::ptr_eq::PtrEqual};

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
        Self::new_with_key(None, widget)
    }

    pub fn new_with_key<W>(key: Option<Key>, widget: W) -> Self
    where
        W: AnyWidget,
    {
        Self {
            key,
            widget: Rc::new(widget),
        }
    }

    pub fn widget_name(&self) -> &str {
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

    pub(crate) fn create_element(&self) -> ElementType {
        Rc::clone(&self.widget).create_element()
    }
}

impl PartialEq for Widget {
    fn eq(&self, other: &Self) -> bool {
        if self.key.is_some() || other.key.is_some() {
            return self.key == other.key;
        }

        self.widget.is_exact_ptr(&other.widget)
    }
}

impl Eq for Widget {}

impl Hash for Widget {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(key) = self.key {
            key.hash(state);

            return;
        }

        // more war crimes
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

#[cfg(test)]
mod tests {
    use std::{ptr, rc::Rc};

    use crate::{
        element::mock::proxy::MockProxyWidget,
        widget::{IntoWidget, Widget},
    };

    #[test]
    fn strip_fat_ptr_equality() {
        let widget1 = MockProxyWidget::default().into_widget();
        let widget2 = MockProxyWidget::default().into_widget();

        // These equality checks are theoretically unstable
        #[allow(clippy::vtable_address_comparisons)]
        {
            assert!(
                !Rc::ptr_eq(&widget1.widget, &widget2.widget),
                "Rc::ptr_eq(widget1, widget2) should never be equal, but is theoretically unstable"
            );

            assert!(
                Rc::ptr_eq(&widget1.widget, &widget1.widget),
                "Rc::ptr_eq(widget1, &widget1) should always be equal, but is theoretically unstable"
            );
        }

        // Black magic fuckery to remove the vtable from the pointer
        assert!(
            !ptr::eq(
                Rc::as_ptr(&widget1.widget) as *const _ as *const (),
                Rc::as_ptr(&widget2.widget) as *const _ as *const ()
            ),
            "ptr::eq(widget1, widget2) should never be equal"
        );

        assert!(
            ptr::eq(
                Rc::as_ptr(&widget1.widget) as *const _ as *const (),
                Rc::as_ptr(&widget1.widget) as *const _ as *const ()
            ),
            "ptr::eq(widget1, widget2) should always be equal"
        );

        // Therefore, this should be stable
        assert_ne!(
            widget1, widget2,
            "widget1 should should never be equal to widget2"
        );

        assert_eq!(
            widget1, widget1,
            "widget1 should should always be equal to itself"
        );
    }

    #[test]
    fn widget_fn_usage() {
        let test1 = IntoWidget::into_widget(widget_test as fn() -> Widget);
        let test2 = IntoWidget::into_widget(widget_test as fn() -> Widget);

        assert_eq!(
            test1, test2,
            "widgets derived from the same function should be equal"
        );

        // This assert isn't true! At higher levels of optimization, the compiler
        // may combine the two functions into one, and therefore the widgets
        // will be equal.
        //
        // let test3 = IntoWidget::into_widget(
        //     (|| MockProxyWidget::default().into_widget()) as fn() -> Widget,
        // );
        //
        // assert_eq!(
        //     test1, test3,
        //     "widgets derived from different functions should not be equal"
        // );
    }

    fn widget_test() -> Widget {
        MockProxyWidget::default().into_widget()
    }
}

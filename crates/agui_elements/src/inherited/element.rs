use std::rc::Rc;

use agui_core::{
    element::{inherited::ElementInherited, widget::ElementWidget, ElementComparison},
    widget::{AnyWidget, Widget},
};

use crate::inherited::InheritedWidget;

pub struct InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    pub(crate) widget: Rc<I>,

    needs_notify: bool,
}

impl<I> InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    pub fn new(widget: Rc<I>) -> Self {
        Self {
            widget,

            needs_notify: false,
        }
    }
}

impl<I> ElementWidget for InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    fn update(&mut self, new_widget: &Widget) -> ElementComparison {
        if let Some(new_widget) = new_widget.downcast::<I>() {
            self.needs_notify |= new_widget.should_notify(self.widget.as_ref());

            self.widget = new_widget;

            // Since (for example) the child of the inherited widget may have changed, we need to
            // rebuild the widget even if we don't need to notify listeners.
            ElementComparison::Changed
        } else {
            ElementComparison::Invalid
        }
    }
}

impl<I> ElementInherited for InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    fn child(&self) -> Widget {
        self.widget.child()
    }

    fn needs_notify(&mut self) -> bool {
        if self.needs_notify {
            self.needs_notify = false;

            true
        } else {
            false
        }
    }
}

impl<I> std::fmt::Debug for InheritedElement<I>
where
    I: AnyWidget + InheritedWidget + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("InheritedElement");

        dbg.field("widget", &self.widget);

        dbg.finish()
    }
}

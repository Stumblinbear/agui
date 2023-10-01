use std::{any::TypeId, rc::Rc};

use crate::{
    inheritance::element::ElementInherited,
    widget::{
        element::{ElementUpdate, ElementWidget},
        AnyWidget, InheritedWidget, Widget,
    },
};

pub struct InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    widget: Rc<I>,

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
    fn widget_name(&self) -> &'static str {
        self.widget.widget_name()
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<I>() {
            self.needs_notify = self.needs_notify || new_widget.should_notify(self.widget.as_ref());

            self.widget = new_widget;

            // Since (for example) the child of the inherited widget may have changed, we need to
            // rebuild the widget even if we don't need to notify listeners.
            ElementUpdate::RebuildNecessary
        } else {
            ElementUpdate::Invalid
        }
    }
}

impl<I> ElementInherited for InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    fn get_inherited_type_id(&self) -> TypeId {
        TypeId::of::<I>()
    }

    fn get_child(&self) -> Widget {
        self.widget.get_child()
    }

    fn should_notify(&mut self) -> bool {
        if self.needs_notify {
            self.needs_notify = false;

            true
        } else {
            false
        }
    }
}

impl<I> InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    pub fn get_inherited_widget(&self) -> Rc<I> {
        Rc::clone(&self.widget)
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

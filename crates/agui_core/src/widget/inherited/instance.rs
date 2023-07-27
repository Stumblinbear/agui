use std::rc::Rc;

use crate::widget::{
    element::{ElementUpdate, WidgetBuildContext, WidgetElement, WidgetMountContext},
    AnyWidget, InheritedWidget, IntoChild, Widget,
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

impl<I> WidgetElement for InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    fn widget_name(&self) -> &'static str {
        self.widget.widget_name()
    }

    fn mount(&mut self, ctx: WidgetMountContext) {
        ctx.inheritance_manager
            .create_scope::<I>(ctx.parent_element_id, ctx.element_id);
    }

    fn build(&mut self, ctx: WidgetBuildContext) -> Vec<Widget> {
        if self.needs_notify {
            self.needs_notify = false;

            // We've been rebuilt and `should_notify` was `true`, so we need to notify all
            // listeners that depend on this widget.
            for element_id in ctx.inheritance_manager.compute_changes::<I>(ctx.element_id) {
                ctx.dirty.insert(element_id);
            }
        }

        Vec::from_iter(self.widget.get_child().into_child())
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<I>() {
            if Rc::ptr_eq(&self.widget, &new_widget) {
                ElementUpdate::Noop
            } else {
                self.needs_notify =
                    self.needs_notify || new_widget.should_notify(self.widget.as_ref());

                self.widget = new_widget;

                ElementUpdate::RebuildNecessary
            }
        } else {
            ElementUpdate::Invalid
        }
    }
}

impl<I> InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    pub fn get_inherited_widget(&self) -> &I {
        &self.widget
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

use std::rc::Rc;

use crate::widget::{
    element::{
        ElementUpdate, WidgetBuildContext, WidgetElement, WidgetMountContext, WidgetUnmountContext,
    },
    AnyWidget, InheritedWidget, IntoChildren, WidgetRef,
};

use super::Inheritance;

pub struct InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    widget: Rc<I>,
}

impl<I> InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    pub fn new(widget: Rc<I>) -> Self {
        Self { widget }
    }
}

impl<I> WidgetElement for InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    fn widget_name(&self) -> &'static str {
        let type_name = self.widget.widget_name();

        type_name
            .split('<')
            .next()
            .unwrap_or(type_name)
            .split("::")
            .last()
            .unwrap_or(type_name)
    }

    fn get_widget(&self) -> Rc<dyn AnyWidget> {
        Rc::clone(&self.widget) as Rc<dyn AnyWidget>
    }

    fn mount(&self, ctx: WidgetMountContext) {
        *ctx.inheritance =
            Inheritance::new_scope::<I>(ctx.inheritance.get_ancestor_scope(), ctx.element_id);
    }

    fn unmount(&self, ctx: WidgetUnmountContext) {}

    fn build(&mut self, ctx: WidgetBuildContext) -> Vec<WidgetRef> {
        let Inheritance::Scope(scope) = ctx.inheritance else {
            panic!("InheritedElement does not have a scope");
        };

        // We've been rebuilt, so we need to notify all listeners that depend on this widget.
        for listener_id in &scope.listeners {
            ctx.dirty.insert(*listener_id);
        }

        self.widget.get_child().into_children()
    }

    fn update(&mut self, new_widget: &WidgetRef) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<I>() {
            if Rc::ptr_eq(&self.widget, &new_widget) {
                ElementUpdate::Noop
            } else {
                let should_notify = new_widget.should_notify(self.widget.as_ref());

                self.widget = new_widget;

                // TODO: fire a notification to all children that depend on this widget

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

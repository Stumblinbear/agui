use std::rc::Rc;

use crate::widget::{
    inheritance::InheritanceScope, InheritedWidget, IntoChildren, WidgetChild, WidgetRef,
};

use super::{AnyWidget, ElementUpdate, ElementWidget, WidgetBuildContext};

pub struct InheritedInstance<W>
where
    W: AnyWidget + WidgetChild + InheritedWidget,
{
    widget: Rc<W>,

    scope: InheritanceScope,
}

impl<W> InheritedInstance<W>
where
    W: AnyWidget + WidgetChild + InheritedWidget,
{
    pub fn new(widget: Rc<W>) -> Self {
        Self {
            widget,

            scope: InheritanceScope::default(),
        }
    }
}

impl<W> ElementWidget for InheritedInstance<W>
where
    W: AnyWidget + WidgetChild + InheritedWidget,
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

    fn build(&mut self, _: WidgetBuildContext) -> Vec<WidgetRef> {
        self.widget.get_child().into_children()
    }

    fn update(&mut self, new_widget: &WidgetRef) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<W>() {
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

impl<W> std::fmt::Debug for InheritedInstance<W>
where
    W: AnyWidget + WidgetChild + InheritedWidget + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("InheritedInstance");

        dbg.field("widget", &self.widget);

        dbg.finish()
    }
}

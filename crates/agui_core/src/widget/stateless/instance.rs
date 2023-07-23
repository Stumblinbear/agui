use std::{marker::PhantomData, rc::Rc};

use fnv::FnvHashMap;

use crate::{
    callback::{CallbackContext, CallbackFunc, CallbackId},
    unit::Data,
    widget::{
        element::{ElementUpdate, WidgetBuildContext, WidgetCallbackContext, WidgetElement},
        AnyWidget, BuildContext, Inheritance, IntoChildren, WidgetRef,
    },
};

use super::WidgetBuild;

pub struct StatelessElement<W>
where
    W: AnyWidget + WidgetBuild,
{
    widget: Rc<W>,

    callbacks: FnvHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,

    inheritance: Inheritance,
}

impl<W> StatelessElement<W>
where
    W: AnyWidget + WidgetBuild,
{
    pub fn new(widget: Rc<W>) -> Self {
        Self {
            widget,

            callbacks: FnvHashMap::default(),

            inheritance: Inheritance::default(),
        }
    }
}

impl<W> WidgetElement for StatelessElement<W>
where
    W: AnyWidget + WidgetBuild,
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

    fn build(&mut self, ctx: WidgetBuildContext) -> Vec<WidgetRef> {
        self.callbacks.clear();

        let mut ctx = BuildContext {
            phantom: PhantomData,

            element_tree: ctx.element_tree,
            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,

            callbacks: &mut self.callbacks,

            inheritance: &mut self.inheritance,
        };

        self.widget.build(&mut ctx).into_children()
    }

    fn update(&mut self, new_widget: &WidgetRef) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<W>() {
            if Rc::ptr_eq(&self.widget, &new_widget) {
                ElementUpdate::Noop
            } else {
                self.widget = new_widget;

                ElementUpdate::RebuildNecessary
            }
        } else {
            ElementUpdate::Invalid
        }
    }

    fn call(
        &mut self,
        ctx: WidgetCallbackContext,
        callback_id: CallbackId,
        arg: &Box<dyn Data>,
    ) -> bool {
        if let Some(callback) = self.callbacks.get(&callback_id) {
            let mut ctx = CallbackContext {
                phantom: PhantomData,

                element_tree: ctx.element_tree,
                dirty: ctx.dirty,

                element_id: ctx.element_id,
            };

            callback.call(&mut ctx, arg);

            false
        } else {
            tracing::warn!(
                callback_id = format!("{:?}", callback_id).as_str(),
                "callback not found"
            );

            false
        }
    }
}

impl<W> std::fmt::Debug for StatelessElement<W>
where
    W: AnyWidget + WidgetBuild + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("StatelessElement");

        dbg.field("widget", &self.widget);

        dbg.finish()
    }
}

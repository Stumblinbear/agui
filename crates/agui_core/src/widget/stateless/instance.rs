use std::{any::Any, marker::PhantomData, rc::Rc};

use rustc_hash::FxHashMap;

use crate::{
    callback::{CallbackContext, CallbackFunc, CallbackId},
    widget::{
        element::{
            ElementBuild, ElementUpdate, ElementWidget, WidgetBuildContext, WidgetCallbackContext,
        },
        widget::Widget,
        AnyWidget, BuildContext,
    },
};

use super::WidgetBuild;

pub struct StatelessElement<W>
where
    W: AnyWidget + WidgetBuild,
{
    widget: Rc<W>,

    callbacks: FxHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,
}

impl<W> StatelessElement<W>
where
    W: AnyWidget + WidgetBuild,
{
    pub fn new(widget: Rc<W>) -> Self {
        Self {
            widget,

            callbacks: FxHashMap::default(),
        }
    }
}

impl<W> ElementWidget for StatelessElement<W>
where
    W: AnyWidget + WidgetBuild,
{
    fn widget_name(&self) -> &'static str {
        self.widget.widget_name()
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<W>() {
            self.widget = new_widget;

            ElementUpdate::RebuildNecessary
        } else {
            ElementUpdate::Invalid
        }
    }
}

impl<W> ElementBuild for StatelessElement<W>
where
    W: AnyWidget + WidgetBuild,
{
    fn build(&mut self, ctx: WidgetBuildContext) -> Widget {
        self.callbacks.clear();

        let mut ctx = BuildContext {
            phantom: PhantomData,

            plugins: ctx.plugins,

            element_tree: ctx.element_tree,

            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,

            callbacks: &mut self.callbacks,
        };

        self.widget.build(&mut ctx)
    }

    fn call(
        &mut self,
        ctx: WidgetCallbackContext,
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    ) -> bool {
        if let Some(callback) = self.callbacks.get(&callback_id) {
            let mut ctx = CallbackContext {
                plugins: ctx.plugins,

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

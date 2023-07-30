use std::{any::Any, rc::Rc};

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::CallbackId,
    widget::{
        element::{ElementUpdate, WidgetBuildContext, WidgetCallbackContext, WidgetElement},
        widget::Widget,
        AnyWidget, IntoChild, StatefulCallbackFunc,
    },
};

use super::{StatefulBuildContext, StatefulCallbackContext, StatefulWidget, WidgetState};

pub struct StatefulElement<W>
where
    W: AnyWidget + StatefulWidget,
{
    widget: Rc<W>,
    state: W::State,

    callbacks: FnvHashMap<CallbackId, Box<dyn StatefulCallbackFunc<W::State>>>,
}

impl<W> StatefulElement<W>
where
    W: AnyWidget + StatefulWidget,
{
    pub fn new(widget: Rc<W>) -> Self {
        let state = widget.create_state();

        Self {
            widget,
            state,

            callbacks: FnvHashMap::default(),
        }
    }
}

impl<W> WidgetElement for StatefulElement<W>
where
    W: AnyWidget + StatefulWidget,
{
    fn widget_name(&self) -> &'static str {
        self.widget.widget_name()
    }

    fn build(&mut self, ctx: WidgetBuildContext) -> Vec<Widget> {
        self.callbacks.clear();

        let mut ctx = StatefulBuildContext {
            element_tree: ctx.element_tree,
            inheritance_manager: ctx.inheritance_manager,

            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,

            callbacks: &mut self.callbacks,

            keyed_children: FnvHashSet::default(),

            widget: &self.widget,
        };

        Vec::from_iter(self.state.build(&mut ctx).into_child())
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<W>() {
            if Rc::ptr_eq(&self.widget, &new_widget) {
                self.state.updated(&new_widget);
            }

            self.widget = new_widget;

            // Stateful widgets always need to be rebuilt because they likely reference widget data
            ElementUpdate::Invalid
        } else {
            ElementUpdate::Invalid
        }
    }

    fn call(
        &mut self,
        ctx: WidgetCallbackContext,
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    ) -> bool {
        if let Some(callback) = self.callbacks.get(&callback_id) {
            let mut ctx = StatefulCallbackContext {
                element_tree: ctx.element_tree,
                inheritance_manager: ctx.inheritance_manager,

                dirty: ctx.dirty,

                element_id: ctx.element_id,

                state: &mut self.state,

                is_changed: false,
            };

            callback.call(&mut ctx, arg);

            ctx.is_changed
        } else {
            tracing::warn!(
                callback_id = format!("{:?}", callback_id).as_str(),
                "callback not found"
            );

            false
        }
    }
}

impl<W> std::fmt::Debug for StatefulElement<W>
where
    W: AnyWidget + StatefulWidget + std::fmt::Debug,
    <W as StatefulWidget>::State: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("StatefulElement");
        dbg.field("widget", &self.widget);
        dbg.field("state", &self.state);
        dbg.finish()
    }
}

use std::rc::Rc;

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::CallbackId,
    unit::AsAny,
    widget::{
        element::{ElementUpdate, WidgetBuildContext, WidgetCallbackContext, WidgetElement},
        AnyWidget, IntoChildren, StatefulCallbackFunc, WidgetRef,
    },
};

use super::{StatefulCallbackContext, StatefulContext, StatefulWidget, WidgetState};

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

        let mut ctx = StatefulContext {
            element_tree: ctx.element_tree,
            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,

            callbacks: &mut self.callbacks,

            inheritance: ctx.inheritance,

            keyed_children: FnvHashSet::default(),

            widget: &self.widget,
            state: &self.state,
        };

        self.state.build(&mut ctx).into_children()
    }

    fn update(&mut self, new_widget: &WidgetRef) -> ElementUpdate {
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
        arg: &Box<dyn AsAny>,
    ) -> bool {
        if let Some(callback) = self.callbacks.get(&callback_id) {
            let mut set_states = Vec::new();

            {
                let mut ctx = StatefulCallbackContext {
                    element_tree: ctx.element_tree,
                    dirty: ctx.dirty,

                    element_id: ctx.element_id,

                    state: &mut self.state,

                    set_states: &mut set_states,
                };

                callback.call(&mut ctx, arg);
            }

            if !set_states.is_empty() {
                for set_state in set_states {
                    set_state(&mut self.state);
                }

                true
            } else {
                false
            }
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

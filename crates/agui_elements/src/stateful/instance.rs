use std::{any::Any, rc::Rc};

use agui_core::{
    callback::CallbackId,
    element::{
        build::ElementBuild, widget::ElementWidget, ElementBuildContext, ElementCallbackContext,
        ElementUpdate,
    },
    widget::{AnyWidget, Widget},
};
use rustc_hash::FxHashMap;

use super::{
    func::StatefulCallbackFunc, StatefulBuildContext, StatefulCallbackContext, StatefulWidget,
    WidgetState,
};

pub struct StatefulElement<W>
where
    W: AnyWidget + StatefulWidget,
{
    widget: Rc<W>,
    state: W::State,

    init_callbacks: FxHashMap<CallbackId, Box<dyn StatefulCallbackFunc<W::State>>>,
    build_callbacks: FxHashMap<CallbackId, Box<dyn StatefulCallbackFunc<W::State>>>,

    initialized: bool,
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

            init_callbacks: FxHashMap::default(),
            build_callbacks: FxHashMap::default(),

            initialized: false,
        }
    }
}

impl<W> ElementWidget for StatefulElement<W>
where
    W: AnyWidget + StatefulWidget,
{
    fn widget_name(&self) -> &'static str {
        self.widget.widget_name()
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<W>() {
            if Rc::ptr_eq(&self.widget, &new_widget) {
                self.state.updated(&new_widget);
            }

            self.widget = new_widget;

            // Stateful widgets always need to be rebuilt because they likely reference widget data
            ElementUpdate::RebuildNecessary
        } else {
            ElementUpdate::Invalid
        }
    }
}

impl<W> ElementBuild for StatefulElement<W>
where
    W: AnyWidget + StatefulWidget,
{
    fn build(&mut self, ctx: ElementBuildContext) -> Widget {
        self.build_callbacks.clear();

        let mut ctx = StatefulBuildContext {
            inner: ctx,

            callbacks: &mut self.build_callbacks,

            widget: &self.widget,
        };

        if !self.initialized {
            let old_callbacks = ctx.callbacks;
            ctx.callbacks = &mut self.init_callbacks;
            {
                self.state.init_state(&mut ctx);

                self.initialized = true;
            }
            ctx.callbacks = old_callbacks;
        }

        self.state.build(&mut ctx)
    }

    fn call(
        &mut self,
        ctx: ElementCallbackContext,
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    ) -> bool {
        if let Some(callback) = self
            .build_callbacks
            .get(&callback_id)
            .or_else(|| self.init_callbacks.get(&callback_id))
        {
            let mut ctx = StatefulCallbackContext {
                inner: ctx,

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

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
    func::StatelessCallbackFunc, StatelessBuildContext, StatelessCallbackContext, StatelessWidget,
};

pub struct StatelessWidgetElement<W>
where
    W: AnyWidget + StatelessWidget,
{
    widget: Rc<W>,

    callbacks: FxHashMap<CallbackId, Box<dyn StatelessCallbackFunc<W>>>,
}

impl<W> StatelessWidgetElement<W>
where
    W: AnyWidget + StatelessWidget,
{
    pub fn new(widget: Rc<W>) -> Self {
        Self {
            widget,

            callbacks: FxHashMap::default(),
        }
    }
}

impl<W> ElementWidget for StatelessWidgetElement<W>
where
    W: AnyWidget + StatelessWidget,
{
    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<W>() {
            self.widget = new_widget;

            ElementUpdate::RebuildNecessary
        } else {
            ElementUpdate::Invalid
        }
    }
}

impl<W> ElementBuild for StatelessWidgetElement<W>
where
    W: AnyWidget + StatelessWidget,
{
    fn build(&mut self, ctx: &mut ElementBuildContext) -> Widget {
        self.callbacks.clear();

        let mut ctx = StatelessBuildContext {
            inner: ctx,

            callbacks: &mut self.callbacks,
        };

        self.widget.build(&mut ctx)
    }

    fn call(
        &mut self,
        ctx: &mut ElementCallbackContext,
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    ) -> bool {
        if let Some(callback) = self.callbacks.get(&callback_id) {
            let mut ctx = StatelessCallbackContext { inner: ctx };

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

impl<W> std::fmt::Debug for StatelessWidgetElement<W>
where
    W: AnyWidget + StatelessWidget + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("StatelessWidgetElement");

        dbg.field("widget", &self.widget);

        dbg.finish()
    }
}

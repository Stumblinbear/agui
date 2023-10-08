use agui_core::{
    callback::{Callback, CallbackId, CallbackQueue, ContextCallbackQueue, WidgetCallback},
    element::{ContextElement, ContextMarkDirty, Element, ElementBuildContext, ElementId},
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    unit::AsAny,
    util::tree::Tree,
};
use rustc_hash::FxHashMap;

use super::{
    func::{StatelessCallbackFn, StatelessCallbackFunc},
    StatelessCallbackContext,
};

pub struct StatelessBuildContext<'ctx, W> {
    pub(crate) inner: ElementBuildContext<'ctx>,

    pub(crate) callbacks: &'ctx mut FxHashMap<CallbackId, Box<dyn StatelessCallbackFunc<W>>>,
}

impl<W> ContextElement for StatelessBuildContext<'_, W> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.inner.get_elements()
    }

    fn get_element_id(&self) -> ElementId {
        self.inner.get_element_id()
    }
}

impl<'ctx, W> ContextPlugins<'ctx> for StatelessBuildContext<'ctx, W> {
    fn get_plugins(&self) -> &Plugins {
        self.inner.get_plugins()
    }
}

impl<'ctx, W> ContextPluginsMut<'ctx> for StatelessBuildContext<'ctx, W> {
    fn get_plugins_mut(&mut self) -> &mut Plugins {
        self.inner.get_plugins_mut()
    }
}

impl<W> ContextMarkDirty for StatelessBuildContext<'_, W> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.inner.mark_dirty(element_id);
    }
}

impl<W> ContextCallbackQueue for StatelessBuildContext<'_, W> {
    fn get_callback_queue(&self) -> &CallbackQueue {
        self.inner.get_callback_queue()
    }
}

impl<W: 'static> StatelessBuildContext<'_, W> {
    pub fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: AsAny,
        F: Fn(&mut StatelessCallbackContext, A) + 'static,
    {
        let callback =
            WidgetCallback::new::<F>(self.get_element_id(), self.get_callback_queue().clone());

        self.callbacks
            .insert(callback.get_id(), Box::new(StatelessCallbackFn::new(func)));

        Callback::Widget(callback)
    }
}

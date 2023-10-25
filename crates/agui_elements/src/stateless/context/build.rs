use std::ops::{Deref, DerefMut};

use agui_core::{
    callback::{Callback, CallbackId, CallbackQueue, ContextCallbackQueue, WidgetCallback},
    element::{
        ContextElement, ContextElements, ContextMarkDirty, Element, ElementBuildContext, ElementId,
    },
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

impl<W> ContextElements for StatelessBuildContext<'_, W> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.inner.elements()
    }
}

impl<W> ContextElement for StatelessBuildContext<'_, W> {
    fn element_id(&self) -> ElementId {
        self.inner.element_id()
    }
}

impl<'ctx, W> ContextPlugins<'ctx> for StatelessBuildContext<'ctx, W> {
    fn plugins(&self) -> &Plugins {
        self.inner.plugins()
    }
}

impl<'ctx, W> ContextPluginsMut<'ctx> for StatelessBuildContext<'ctx, W> {
    fn plugins_mut(&mut self) -> &mut Plugins {
        self.inner.plugins_mut()
    }
}

impl<W> ContextMarkDirty for StatelessBuildContext<'_, W> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.inner.mark_dirty(element_id);
    }
}

impl<W> ContextCallbackQueue for StatelessBuildContext<'_, W> {
    fn callback_queue(&self) -> &CallbackQueue {
        self.inner.callback_queue()
    }
}

impl<'ctx, W> Deref for StatelessBuildContext<'ctx, W> {
    type Target = ElementBuildContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'ctx, W> DerefMut for StatelessBuildContext<'ctx, W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<W: 'static> StatelessBuildContext<'_, W> {
    pub fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: AsAny,
        F: Fn(&mut StatelessCallbackContext, A) + 'static,
    {
        let callback = WidgetCallback::new::<F>(self.element_id(), self.callback_queue().clone());

        self.callbacks
            .insert(callback.id(), Box::new(StatelessCallbackFn::new(func)));

        Callback::Widget(callback)
    }
}

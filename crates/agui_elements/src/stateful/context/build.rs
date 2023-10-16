use std::ops::{Deref, DerefMut};

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

use crate::stateful::WidgetState;

use super::{
    func::{StatefulCallbackFn, StatefulCallbackFunc},
    StatefulCallbackContext,
};

pub struct StatefulBuildContext<'ctx, S>
where
    S: WidgetState + ?Sized,
{
    pub(crate) inner: ElementBuildContext<'ctx>,

    pub(crate) callbacks: &'ctx mut FxHashMap<CallbackId, Box<dyn StatefulCallbackFunc<S>>>,

    pub widget: &'ctx S::Widget,
}

impl<S> ContextElement for StatefulBuildContext<'_, S>
where
    S: WidgetState,
{
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.inner.get_elements()
    }

    fn get_element_id(&self) -> ElementId {
        self.inner.get_element_id()
    }
}

impl<'ctx, S> ContextPlugins<'ctx> for StatefulBuildContext<'ctx, S>
where
    S: WidgetState,
{
    fn get_plugins(&self) -> &Plugins {
        self.inner.get_plugins()
    }
}

impl<'ctx, S> ContextPluginsMut<'ctx> for StatefulBuildContext<'ctx, S>
where
    S: WidgetState,
{
    fn get_plugins_mut(&mut self) -> &mut Plugins {
        self.inner.get_plugins_mut()
    }
}

impl<S> ContextMarkDirty for StatefulBuildContext<'_, S>
where
    S: WidgetState,
{
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.inner.mark_dirty(element_id)
    }
}

impl<S> ContextCallbackQueue for StatefulBuildContext<'_, S>
where
    S: WidgetState,
{
    fn get_callback_queue(&self) -> &CallbackQueue {
        self.inner.get_callback_queue()
    }
}

impl<'ctx, S: 'static> Deref for StatefulBuildContext<'ctx, S>
where
    S: WidgetState,
{
    type Target = ElementBuildContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'ctx, S: 'static> DerefMut for StatefulBuildContext<'ctx, S>
where
    S: WidgetState,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'ctx, S: 'static> StatefulBuildContext<'ctx, S>
where
    S: WidgetState,
{
    pub fn get_widget(&self) -> &S::Widget {
        self.widget
    }

    pub fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: AsAny,
        F: Fn(&mut StatefulCallbackContext<S>, A) + 'static,
    {
        let callback =
            WidgetCallback::new::<F>(self.get_element_id(), self.get_callback_queue().clone());

        self.callbacks
            .insert(callback.get_id(), Box::new(StatefulCallbackFn::new(func)));

        Callback::Widget(callback)
    }
}

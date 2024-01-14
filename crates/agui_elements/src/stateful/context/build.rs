use std::ops::{Deref, DerefMut};

use agui_core::{
    callback::{Callback, CallbackId, CallbackQueue, ContextCallbackQueue, WidgetCallback},
    element::{ContextElement, ContextElements, Element, ElementBuildContext, ElementId},
    unit::AsAny,
    util::tree::Tree,
};
use rustc_hash::FxHashMap;

use crate::stateful::WidgetState;

use super::{
    func::{StatefulCallbackFn, StatefulCallbackFunc},
    StatefulCallbackContext,
};

pub struct StatefulBuildContext<'ctx, 'element, S>
where
    S: WidgetState + ?Sized,
{
    pub(crate) inner: &'element mut ElementBuildContext<'ctx>,

    pub(crate) callbacks: &'element mut FxHashMap<CallbackId, Box<dyn StatefulCallbackFunc<S>>>,

    pub widget: &'element S::Widget,
}

impl<S> ContextElements for StatefulBuildContext<'_, '_, S>
where
    S: WidgetState,
{
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.inner.elements()
    }
}

impl<S> ContextElement for StatefulBuildContext<'_, '_, S>
where
    S: WidgetState,
{
    fn element_id(&self) -> ElementId {
        self.inner.element_id()
    }
}

impl<S> ContextCallbackQueue for StatefulBuildContext<'_, '_, S>
where
    S: WidgetState,
{
    fn callback_queue(&self) -> &CallbackQueue {
        self.inner.callback_queue()
    }
}

impl<'ctx, S: 'static> Deref for StatefulBuildContext<'ctx, '_, S>
where
    S: WidgetState,
{
    type Target = ElementBuildContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'ctx, S: 'static> DerefMut for StatefulBuildContext<'ctx, '_, S>
where
    S: WidgetState,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl<'ctx, S: 'static> StatefulBuildContext<'ctx, '_, S>
where
    S: WidgetState,
{
    pub fn widget(&self) -> &S::Widget {
        self.widget
    }

    pub fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: AsAny,
        F: Fn(&mut StatefulCallbackContext<S>, A) + 'static,
    {
        let callback = WidgetCallback::new::<F>(self.element_id(), self.callback_queue().clone());

        self.callbacks
            .insert(callback.id(), Box::new(StatefulCallbackFn::new(func)));

        Callback::Widget(callback)
    }
}

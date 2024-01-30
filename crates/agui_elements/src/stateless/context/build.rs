use std::{
    any::Any,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use agui_core::{
    callback::{Callback, CallbackId},
    element::{ContextElement, ContextElements, Element, ElementBuildContext, ElementId},
    util::tree::Tree,
};
use rustc_hash::FxHashMap;

use super::{
    func::{StatelessCallbackFn, StatelessCallbackFunc},
    StatelessCallbackContext,
};

pub struct StatelessBuildContext<'ctx, 'element, W> {
    pub(crate) inner: &'element mut ElementBuildContext<'ctx>,

    pub(crate) callbacks: &'element mut FxHashMap<CallbackId, Box<dyn StatelessCallbackFunc<W>>>,
}

impl<W> ContextElements for StatelessBuildContext<'_, '_, W> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.inner.elements()
    }
}

impl<W> ContextElement for StatelessBuildContext<'_, '_, W> {
    fn element_id(&self) -> ElementId {
        self.inner.element_id()
    }
}

impl<'ctx, W> Deref for StatelessBuildContext<'ctx, '_, W> {
    type Target = ElementBuildContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'ctx, W> DerefMut for StatelessBuildContext<'ctx, '_, W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl<W> StatelessBuildContext<'_, '_, W>
where
    W: Any,
{
    pub fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: Any,
        F: Fn(&mut StatelessCallbackContext, A) + 'static,
    {
        let callback = Callback::new::<F>(Arc::clone(self.inner.callbacks), *self.element_id);

        self.callbacks
            .insert(callback.id(), Box::new(StatelessCallbackFn::new(func)));

        callback
    }
}

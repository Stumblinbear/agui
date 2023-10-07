use std::marker::PhantomData;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    callback::{
        Callback, CallbackContext, CallbackFn, CallbackFunc, CallbackId, CallbackQueue,
        WidgetCallback,
    },
    element::{Element, ElementId},
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    unit::AsAny,
    util::tree::Tree,
    widget::{ContextElement, ContextMarkDirty},
};

pub struct BuildContext<'ctx, W> {
    pub(crate) phantom: PhantomData<W>,

    pub(crate) plugins: Plugins<'ctx>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,

    pub(crate) callbacks: &'ctx mut FxHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,
}

impl<W> ContextElement for BuildContext<'_, W> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<'ctx, W> ContextPlugins<'ctx> for BuildContext<'ctx, W> {
    fn get_plugins(&self) -> &Plugins<'ctx> {
        &self.plugins
    }
}

impl<'ctx, W> ContextPluginsMut<'ctx> for BuildContext<'ctx, W> {
    fn get_plugins_mut(&mut self) -> &mut Plugins<'ctx> {
        &mut self.plugins
    }
}

impl<W> ContextMarkDirty for BuildContext<'_, W> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

impl<W: 'static> BuildContext<'_, W> {
    pub fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: AsAny,
        F: Fn(&mut CallbackContext, A) + 'static,
    {
        let callback = WidgetCallback::new::<F>(self.element_id, self.callback_queue.clone());

        self.callbacks
            .insert(callback.get_id(), Box::new(CallbackFn::new(func)));

        Callback::Widget(callback)
    }
}

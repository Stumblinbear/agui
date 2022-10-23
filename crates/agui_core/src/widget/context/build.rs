use std::ops::Deref;

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::{Callback, CallbackContext, CallbackFn, CallbackFunc, CallbackId, CallbackQueue},
    manager::element::WidgetElement,
    plugin::{BoxedPlugin, PluginElement, PluginId, PluginImpl},
    unit::{Data, Key},
    util::{map::PluginMap, tree::Tree},
    widget::{Widget, WidgetId, WidgetKey, WidgetRef, WidgetState, WidgetView},
};

use super::{ContextMut, ContextPlugins, ContextStatefulWidget, ContextWidget, ContextWidgetMut};

pub struct BuildContext<'ctx, W>
where
    W: WidgetView + WidgetState,
{
    pub(crate) plugins: &'ctx mut PluginMap<BoxedPlugin>,
    pub(crate) widget_tree: &'ctx Tree<WidgetId, WidgetElement>,
    pub(crate) dirty: &'ctx mut FnvHashSet<WidgetId>,
    pub(crate) callback_queue: CallbackQueue,

    pub(crate) widget_id: WidgetId,
    pub widget: &'ctx W,
    pub state: &'ctx mut W::State,

    pub(crate) callbacks: FnvHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,
}

impl<W> Deref for BuildContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.widget
    }
}

impl<W> ContextPlugins for BuildContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    fn get_plugins(&mut self) -> &mut PluginMap<BoxedPlugin> {
        self.plugins
    }

    fn get_plugin<P>(&self) -> Option<&PluginElement<P>>
    where
        P: PluginImpl,
    {
        self.plugins
            .get(&PluginId::of::<P>())
            .and_then(|p| p.downcast_ref())
    }

    fn get_plugin_mut<P>(&mut self) -> Option<&mut PluginElement<P>>
    where
        P: PluginImpl,
    {
        self.plugins
            .get_mut(&PluginId::of::<P>())
            .and_then(|p| p.downcast_mut())
    }
}

impl<W> ContextMut for BuildContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    fn mark_dirty(&mut self, widget_id: WidgetId) {
        self.dirty.insert(widget_id);
    }

    fn call<A>(&mut self, callback: &Callback<A>, arg: A)
    where
        A: Data,
    {
        self.callback_queue.call(callback, arg);
    }

    unsafe fn call_unsafe(&mut self, callback_id: CallbackId, arg: Box<dyn Data>) {
        self.callback_queue.call_unsafe(callback_id, arg);
    }

    fn call_many<A>(&mut self, callbacks: &[Callback<A>], arg: A)
    where
        A: Data,
    {
        self.callback_queue.call_many(callbacks, arg);
    }

    unsafe fn call_many_unsafe(&mut self, callback_ids: &[CallbackId], arg: Box<dyn Data>) {
        self.callback_queue.call_many_unsafe(callback_ids, arg);
    }
}

impl<W> ContextWidget for BuildContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    type Widget = W;

    fn get_widgets(&self) -> &Tree<WidgetId, WidgetElement> {
        self.widget_tree
    }

    fn get_widget_id(&self) -> WidgetId {
        self.widget_id
    }

    fn get_widget(&self) -> &W {
        self.widget
    }
}

impl<W> ContextStatefulWidget for BuildContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    fn get_state(&self) -> &W::State {
        self.state
    }

    fn get_state_mut(&mut self) -> &mut W::State {
        self.state
    }

    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut W::State),
    {
        func(self.state);
    }
}

impl<W> ContextWidgetMut for BuildContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: Data,
        F: Fn(&mut CallbackContext<W>, &A) + 'static,
    {
        let callback = Callback::new::<F, W>(self.widget_id, self.callback_queue.clone());

        self.callbacks
            .insert(callback.get_id().unwrap(), Box::new(CallbackFn::new(func)));

        callback
    }
}

impl<W> BuildContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    pub fn key<C>(&self, key: Key, widget: C) -> WidgetRef
    where
        C: Widget,
    {
        WidgetRef::new_with_key(
            Some(match key {
                Key::Local(_) => WidgetKey(Some(self.widget_id), key),
                Key::Global(_) => WidgetKey(None, key),
            }),
            widget,
        )
    }
}

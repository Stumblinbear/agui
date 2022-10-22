use std::ops::Deref;

use crate::{
    manager::element::WidgetElement,
    plugin::{BoxedPlugin, PluginElement, PluginId, PluginImpl},
    unit::Data,
    util::{
        map::{PluginMap, WidgetSet},
        tree::Tree,
    },
    widget::{
        ContextMut, ContextPlugins, ContextStatefulWidget, ContextWidget, Widget, WidgetId,
        WidgetState,
    },
};

use super::{Callback, CallbackId, CallbackQueue};

pub struct CallbackContext<'ctx, W>
where
    W: Widget + WidgetState,
{
    pub(crate) plugins: &'ctx mut PluginMap<BoxedPlugin>,
    pub(crate) widget_tree: &'ctx Tree<WidgetId, WidgetElement>,
    pub(crate) dirty: &'ctx mut WidgetSet,
    pub(crate) callback_queue: CallbackQueue,

    pub(crate) widget_id: WidgetId,
    pub widget: &'ctx W,
    pub state: &'ctx mut W::State,

    pub(crate) changed: bool,
}

impl<W> Deref for CallbackContext<'_, W>
where
    W: Widget + WidgetState,
{
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.widget
    }
}

impl<W> ContextPlugins for CallbackContext<'_, W>
where
    W: Widget + WidgetState,
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

impl<W> ContextMut for CallbackContext<'_, W>
where
    W: Widget + WidgetState,
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

impl<W> ContextWidget for CallbackContext<'_, W>
where
    W: Widget + WidgetState,
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

impl<W> ContextStatefulWidget for CallbackContext<'_, W>
where
    W: Widget + WidgetState,
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
        self.changed = true;

        func(self.state);
    }
}

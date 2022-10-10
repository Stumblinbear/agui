use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::{Callback, CallbackContext, CallbackFn, CallbackFunc, CallbackId, CallbackQueue},
    manager::element::WidgetElement,
    plugin::{BoxedPlugin, PluginElement, PluginId, PluginImpl},
    render::{
        canvas::painter::{CanvasPainter, Head},
        context::RenderContext,
        renderer::RenderFn,
    },
    unit::{Data, Key},
    util::{map::PluginMap, tree::Tree},
    widget::{IntoWidget, WidgetBuilder, WidgetId, WidgetKey, WidgetRef},
};

use super::WidgetContext;

pub struct BuildContext<'ctx, W>
where
    W: WidgetBuilder,
{
    pub(crate) plugins: &'ctx mut PluginMap<BoxedPlugin>,
    pub(crate) widget_tree: &'ctx Tree<WidgetId, WidgetElement>,
    pub(crate) dirty: &'ctx mut FnvHashSet<WidgetId>,
    pub(crate) callback_queue: CallbackQueue,

    pub(crate) widget_id: WidgetId,
    pub widget: &'ctx W,
    pub state: &'ctx mut W::State,

    pub(crate) renderer: Option<RenderFn<W>>,
    pub(crate) callbacks: FnvHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,
}

impl<W> WidgetContext<W> for BuildContext<'_, W>
where
    W: WidgetBuilder,
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

    fn get_widgets(&self) -> &Tree<WidgetId, WidgetElement> {
        self.widget_tree
    }

    fn mark_dirty(&mut self, widget_id: WidgetId) {
        self.dirty.insert(widget_id);
    }

    // fn depend_on<D>(&mut self) -> Option<&D::State>
    // where
    //     D: WidgetBuilder,
    // {
    //     self.inherited
    //         .get(&TypeId::of::<D>())
    //         .and_then(|widget_id| {
    //             self.widget_tree
    //                 .get(*widget_id)
    //                 .and_then(|widget| widget.downcast_ref::<WidgetInstance<D>>())
    //                 .map(|element| element.get_state())
    //         })
    // }

    fn get_widget(&self) -> &W {
        self.widget
    }

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

impl<W> BuildContext<'_, W>
where
    W: WidgetBuilder,
{
    pub fn get_widget_id(&self) -> WidgetId {
        self.widget_id
    }

    pub fn on_draw<F>(&mut self, func: F)
    where
        F: Fn(&RenderContext<W>, CanvasPainter<Head>) + 'static,
    {
        self.renderer = Some(RenderFn::new(func));
    }

    pub fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: Data,
        F: Fn(&mut CallbackContext<W>, &A) + 'static,
    {
        let callback = Callback::new::<F, W>(self.widget_id, self.callback_queue.clone());

        self.callbacks
            .insert(callback.get_id().unwrap(), Box::new(CallbackFn::new(func)));

        callback
    }

    pub fn key<C>(&self, key: Key, widget: C) -> WidgetRef
    where
        C: IntoWidget,
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

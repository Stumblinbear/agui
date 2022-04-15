use std::{rc::Rc, sync::Arc};

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::{Callback, CallbackContext, CallbackFn, CallbackFunc, CallbackId},
    canvas::{renderer::RenderFn, Canvas},
    engine::{context::Context, tree::Tree, Data, NotifyCallback},
    plugin::{EnginePlugin, Plugin, PluginId, PluginMut, PluginRef},
    unit::{Key, Layout, LayoutType, Rect, Size},
    widget::WidgetId,
};

use super::{Widget, WidgetKey};

pub struct BuildContext<'ctx, S>
where
    S: Data,
{
    pub(crate) plugins: &'ctx mut FnvHashMap<PluginId, Plugin>,
    pub(crate) tree: &'ctx Tree<WidgetId, Widget>,
    pub(crate) dirty: &'ctx mut FnvHashSet<WidgetId>,
    pub(crate) notifier: NotifyCallback,

    pub(crate) widget_id: WidgetId,
    pub(crate) state: &'ctx mut S,

    pub(crate) layout_type: LayoutType,
    pub(crate) layout: Layout,

    pub(crate) rect: Option<Rect>,

    pub(crate) renderer: Option<RenderFn>,
    pub(crate) callbacks: FnvHashMap<CallbackId, Box<dyn CallbackFunc<S>>>,
}

impl<S> Context<S> for BuildContext<'_, S>
where
    S: Data,
{
    fn get_plugins(&mut self) -> &mut FnvHashMap<PluginId, Plugin> {
        self.plugins
    }

    fn get_plugin<P>(&self) -> Option<PluginRef<P>>
    where
        P: EnginePlugin,
    {
        self.plugins
            .get(&PluginId::of::<P>())
            .map(|p| p.get_as::<P>().unwrap())
    }

    fn get_plugin_mut<P>(&mut self) -> Option<PluginMut<P>>
    where
        P: EnginePlugin,
    {
        self.plugins
            .get_mut(&PluginId::of::<P>())
            .map(|p| p.get_as_mut::<P>().unwrap())
    }

    fn get_tree(&self) -> &Tree<WidgetId, Widget> {
        self.tree
    }

    fn mark_dirty(&mut self, widget_id: WidgetId) {
        self.dirty.insert(widget_id);
    }

    fn get_rect(&self) -> Option<Rect> {
        self.rect
    }

    fn get_size(&self) -> Option<Size> {
        self.rect.map(|rect| rect.into())
    }

    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut S),
    {
        func(self.state);
    }

    fn get_state(&self) -> &S
    where
        S: Data,
    {
        self.state
    }

    fn get_state_mut(&mut self) -> &mut S
    where
        S: Data,
    {
        self.state
    }

    fn notify<A>(&mut self, callback_id: CallbackId, args: A)
    where
        A: Data,
    {
        self.notifier.lock().push((callback_id, Rc::new(args)));
    }

    /// # Safety
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    unsafe fn notify_unsafe(&mut self, callback_id: CallbackId, args: Rc<dyn Data>) {
        self.notifier.lock().push((callback_id, args));
    }
}

impl<S> BuildContext<'_, S>
where
    S: Data,
{
    pub fn get_widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Set the layout type of the widget.
    pub fn set_layout_type(&mut self, layout_type: LayoutType) {
        self.layout_type = layout_type;
    }

    /// Set the layout of the widget.
    pub fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
    }

    pub fn on_draw<F>(&mut self, func: F)
    where
        F: Fn(&mut Canvas) + 'static,
    {
        self.renderer = Some(RenderFn::new(func));
    }

    pub fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: Data,
        F: Fn(&mut CallbackContext<S>, &A) + 'static,
    {
        let callback = Callback::new::<F, S>(Arc::clone(&self.notifier), self.widget_id);

        self.callbacks
            .insert(callback.get_id().unwrap(), Box::new(CallbackFn::new(func)));

        callback
    }

    pub fn key(&self, key: Key, widget: &Widget) -> Widget {
        if widget.get_key().is_some() {
            panic!("cannot key a widget that has already been keyed");
        }

        let mut widget = widget.clone();

        if let Widget::Some {
            key: widget_key, ..
        } = &mut widget
        {
            *widget_key = Some(match key {
                Key::Local(_) => WidgetKey(Some(self.widget_id), key),
                Key::Global(_) => WidgetKey(None, key),
            });
        }

        widget
    }
}

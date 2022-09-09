use std::{rc::Rc, sync::Arc};

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::{Callback, CallbackContext, CallbackFn, CallbackFunc, CallbackId, CallbackQueue},
    plugin::{BoxedPlugin, PluginElement, PluginId, PluginImpl},
    render::{canvas::painter::CanvasPainter, context::RenderContext, renderer::RenderFn},
    unit::{Data, Key, Layout, LayoutType, Rect, Size},
    util::{map::PluginMap, tree::Tree},
    widget::{BoxedWidget, IntoWidget, Widget, WidgetBuilder, WidgetId, WidgetKey},
};

use super::WidgetContext;

pub struct BuildContext<'ctx, W>
where
    W: WidgetBuilder,
{
    pub(crate) plugins: &'ctx mut PluginMap<BoxedPlugin>,
    pub(crate) tree: &'ctx Tree<WidgetId, BoxedWidget>,
    pub(crate) dirty: &'ctx mut FnvHashSet<WidgetId>,
    pub(crate) callback_queue: CallbackQueue,

    pub(crate) widget_id: WidgetId,
    pub widget: &'ctx W,
    pub state: &'ctx mut W::State,

    pub(crate) layout_type: LayoutType,
    pub(crate) layout: Layout,

    pub(crate) rect: Option<Rect>,

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

    fn get_tree(&self) -> &Tree<WidgetId, BoxedWidget> {
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

    fn call<A>(&mut self, callback: Callback<A>, args: A)
    where
        A: Data,
    {
        if let Some(callback_id) = callback.get_id() {
            self.callback_queue
                .lock()
                .push((callback_id, Rc::new(args)));
        }
    }

    /// # Safety
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    unsafe fn call_unsafe(&mut self, callback_id: CallbackId, args: Rc<dyn Data>) {
        self.callback_queue.lock().push((callback_id, args));
    }
}

impl<W> BuildContext<'_, W>
where
    W: WidgetBuilder,
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
        F: Fn(&RenderContext<W>, CanvasPainter) + 'static,
    {
        self.renderer = Some(RenderFn::new(func));
    }

    pub fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: Data,
        F: Fn(&mut CallbackContext<W>, &A) + 'static,
    {
        let callback = Callback::new::<F, W>(self.widget_id, Arc::clone(&self.callback_queue));

        self.callbacks
            .insert(callback.get_id().unwrap(), Box::new(CallbackFn::new(func)));

        callback
    }

    pub fn key<C>(&self, key: Key, widget: C) -> Widget
    where
        C: IntoWidget + 'static,
    {
        let mut widget = Widget::from(widget);

        widget.set_key(match key {
            Key::Local(_) => WidgetKey(Some(self.widget_id), key),
            Key::Global(_) => WidgetKey(None, key),
        });

        widget
    }
}

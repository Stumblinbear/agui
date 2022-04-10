use std::any::TypeId;

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::{Callback, CallbackContext, CallbackFn, CallbackFunc, CallbackId},
    canvas::{renderer::RenderFn, Canvas},
    engine::{tree::Tree, Data},
    plugin::{Plugin, PluginId},
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

    pub(crate) widget_id: WidgetId,
    pub(crate) state: &'ctx mut S,

    pub(crate) layout_type: LayoutType,
    pub(crate) layout: Layout,

    pub(crate) renderer: Option<RenderFn>,
    pub(crate) callbacks: FnvHashMap<CallbackId, Box<dyn CallbackFunc<S>>>,

    pub(crate) rect: Option<Rect>,
}

impl<S> BuildContext<'_, S>
where
    S: Data,
{
    pub fn get_plugins(&mut self) -> &mut FnvHashMap<PluginId, Plugin> {
        self.plugins
    }

    pub fn get_tree(&self) -> &Tree<WidgetId, Widget> {
        self.tree
    }

    pub fn mark_dirty(&mut self, widget_id: WidgetId) {
        self.dirty.insert(widget_id);
    }

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

    pub fn get_rect(&self) -> Option<Rect> {
        self.rect
    }

    pub fn get_size(&self) -> Option<Size> {
        self.rect.map(|rect| rect.into())
    }

    pub fn key(&self, key: Key, widget: &Widget) -> Widget {
        widget.with_key(match key {
            Key::Local(_) => WidgetKey(Some(self.widget_id), key),
            Key::Global(_) => WidgetKey(None, key),
        })
    }

    pub fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut S) + 'static,
    {
        func(self.state);
    }

    pub fn get_state(&self) -> &S
    where
        S: Data,
    {
        self.state
    }

    pub fn get_state_mut(&mut self) -> &mut S
    where
        S: Data,
    {
        self.state
    }

    pub fn callback<F, A>(&mut self, func: F) -> Callback<A>
    where
        F: Fn(&mut CallbackContext<S>, &A) + 'static,
        A: Data + Clone,
    {
        let callback_id = CallbackId(self.widget_id, TypeId::of::<F>());

        let callback = Callback::new(callback_id);

        self.callbacks
            .insert(callback_id, Box::new(CallbackFn::new(func)));

        callback
    }
}

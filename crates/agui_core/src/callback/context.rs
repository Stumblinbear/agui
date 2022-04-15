use std::{ops::Deref, rc::Rc};

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    engine::{context::Context, tree::Tree, Data, NotifyCallback},
    plugin::{EnginePlugin, Plugin, PluginId, PluginMut, PluginRef},
    unit::{Rect, Size},
    widget::{Widget, WidgetId},
};

use super::CallbackId;

pub struct CallbackContext<'ctx, S>
where
    S: Data,
{
    pub(crate) plugins: &'ctx mut FnvHashMap<PluginId, Plugin>,
    pub(crate) tree: &'ctx Tree<WidgetId, Widget>,
    pub(crate) dirty: &'ctx mut FnvHashSet<WidgetId>,
    pub(crate) notifier: NotifyCallback,

    pub(crate) state: &'ctx mut S,

    pub(crate) rect: Option<Rect>,

    pub(crate) changed: bool,
}

impl<S> Deref for CallbackContext<'_, S>
where
    S: Data,
{
    type Target = S;

    fn deref(&self) -> &Self::Target {
        self.state
    }
}

impl<S> Context<S> for CallbackContext<'_, S>
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
        self.changed = true;

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

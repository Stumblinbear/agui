use std::{ops::Deref, rc::Rc};

use crate::{
    manager::{context::Context, CallbackQueue, Data},
    plugin::{BoxedPlugin, PluginElement, PluginId, PluginImpl},
    unit::{Rect, Size},
    util::{
        map::{PluginMap, WidgetSet},
        tree::Tree,
    },
    widget::{BoxedWidget, WidgetBuilder, WidgetId},
};

use super::{Callback, CallbackId};

pub struct CallbackContext<'ctx, W>
where
    W: WidgetBuilder,
{
    pub(crate) plugins: &'ctx mut PluginMap<BoxedPlugin>,
    pub(crate) tree: &'ctx Tree<WidgetId, BoxedWidget>,
    pub(crate) dirty: &'ctx mut WidgetSet,
    pub(crate) callback_queue: CallbackQueue,

    pub widget: &'ctx W,
    pub state: &'ctx mut W::State,

    pub rect: Option<Rect>,

    pub(crate) changed: bool,
}

impl<W> Deref for CallbackContext<'_, W>
where
    W: WidgetBuilder,
{
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.widget
    }
}

impl<W> Context<W> for CallbackContext<'_, W>
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
        self.changed = true;

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

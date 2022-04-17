use std::rc::Rc;

use fnv::FnvHashSet;

use crate::{
    callback::{Callback, CallbackId},
    plugin::{EnginePlugin, Plugin, PluginMut, PluginRef},
    unit::{Rect, Size},
    util::map::PluginMap,
    widget::{Widget, WidgetId},
};

use super::{tree::Tree, widget::WidgetBuilder, ArcEmitCallbacks, Data, EmitCallbacks};

pub struct EngineContext<'ctx> {
    pub(crate) plugins: Option<&'ctx mut PluginMap<Plugin>>,
    pub(crate) tree: &'ctx Tree<WidgetId, Widget>,
    pub(crate) dirty: &'ctx mut FnvHashSet<WidgetId>,

    pub(crate) emit_callbacks: &'ctx mut EmitCallbacks,
    pub(crate) arc_emit_callbacks: ArcEmitCallbacks,
}

pub trait Context<W>
where
    W: WidgetBuilder,
{
    fn get_plugins(&mut self) -> &mut PluginMap<Plugin>;

    fn get_plugin<P>(&self) -> Option<PluginRef<P>>
    where
        P: EnginePlugin;

    fn get_plugin_mut<P>(&mut self) -> Option<PluginMut<P>>
    where
        P: EnginePlugin;

    fn get_tree(&self) -> &Tree<WidgetId, Widget>;

    fn mark_dirty(&mut self, widget_id: WidgetId);

    fn get_rect(&self) -> Option<Rect>;

    fn get_size(&self) -> Option<Size> {
        self.get_rect().map(|rect| rect.into())
    }

    fn get_widget(&self) -> &W;

    fn get_state(&self) -> &W::State;

    fn get_state_mut(&mut self) -> &mut W::State;

    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut W::State);

    fn emit<A>(&mut self, callback: Callback<A>, args: A)
    where
        A: Data;

    /// # Safety
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    unsafe fn emit_unsafe(&mut self, callback_id: CallbackId, args: Rc<dyn Data>);
}

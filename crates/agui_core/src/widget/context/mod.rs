use std::rc::Rc;

use crate::{
    callback::{Callback, CallbackId},
    plugin::{BoxedPlugin, PluginElement, PluginImpl},
    unit::{Data, Rect, Size},
    util::{map::PluginMap, tree::Tree},
    widget::{BoxedWidget, WidgetBuilder, WidgetId},
};

mod build;

pub use build::*;

pub trait WidgetContext<W>
where
    W: WidgetBuilder,
{
    fn get_plugins(&mut self) -> &mut PluginMap<BoxedPlugin>;

    fn get_plugin<P>(&self) -> Option<&PluginElement<P>>
    where
        P: PluginImpl;

    fn get_plugin_mut<P>(&mut self) -> Option<&mut PluginElement<P>>
    where
        P: PluginImpl;

    fn get_tree(&self) -> &Tree<WidgetId, BoxedWidget>;

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

    fn call<A>(&mut self, callback: Callback<A>, args: A)
    where
        A: Data;

    /// # Safety
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    unsafe fn call_unsafe(&mut self, callback_id: CallbackId, args: Rc<dyn Data>);
}

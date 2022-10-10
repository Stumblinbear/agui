use crate::{
    callback::{Callback, CallbackId},
    manager::widgets::element::WidgetElement,
    plugin::{BoxedPlugin, PluginElement, PluginImpl},
    unit::Data,
    util::{map::PluginMap, tree::Tree},
    widget::{WidgetBuilder, WidgetId},
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

    fn get_widgets(&self) -> &Tree<WidgetId, WidgetElement>;

    fn mark_dirty(&mut self, widget_id: WidgetId);

    // fn depend_on<D>(&mut self) -> Option<&D::State>
    // where
    //     D: WidgetBuilder;

    fn get_widget(&self) -> &W;

    fn get_state(&self) -> &W::State;

    fn get_state_mut(&mut self) -> &mut W::State;

    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut W::State);

    fn call<A>(&mut self, callback: Callback<A>, arg: A)
    where
        A: Data;

    /// # Safety
    ///
    /// You must ensure the callback is expecting the type of the `arg` passed in. If the type
    /// is different, it will panic.
    unsafe fn call_unsafe(&mut self, callback_id: CallbackId, arg: Box<dyn Data>);

    fn call_many<A>(&mut self, callbacks: &[Callback<A>], arg: A)
    where
        A: Data;

    /// # Safety
    ///
    /// You must ensure the callbacks are expecting the type of the `arg` passed in. If the type
    /// is different, it will panic.
    unsafe fn call_many_unsafe(&mut self, callback_ids: &[CallbackId], arg: Box<dyn Data>);
}

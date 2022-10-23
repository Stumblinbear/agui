use crate::{
    callback::{Callback, CallbackContext, CallbackId},
    manager::element::WidgetElement,
    plugin::{BoxedPlugin, PluginElement, PluginImpl},
    unit::Data,
    util::{map::PluginMap, tree::Tree},
    widget::{Widget, WidgetId},
};

mod build;
mod layout;
mod paint;

pub use build::*;
pub use layout::*;
pub use paint::*;

use super::WidgetState;

pub trait ContextMut {
    fn mark_dirty(&mut self, widget_id: WidgetId);

    fn call<A>(&mut self, callback: &Callback<A>, arg: A)
    where
        A: Data;

    /// # Panics
    ///
    /// You must ensure the callback is expecting the type of the `arg` passed in. If the type
    /// is different, it will panic.
    fn call_unchecked(&mut self, callback_id: CallbackId, arg: Box<dyn Data>);

    fn call_many<A>(&mut self, callbacks: &[Callback<A>], arg: A)
    where
        A: Data;

    /// # Panics
    ///
    /// You must ensure the callbacks are expecting the type of the `arg` passed in. If the type
    /// is different, it will panic.
    fn call_many_unchecked(&mut self, callback_ids: &[CallbackId], arg: Box<dyn Data>);
}

pub trait ContextPlugins {
    fn get_plugins(&mut self) -> &mut PluginMap<BoxedPlugin>;

    fn get_plugin<P>(&self) -> Option<&PluginElement<P>>
    where
        P: PluginImpl;

    fn get_plugin_mut<P>(&mut self) -> Option<&mut PluginElement<P>>
    where
        P: PluginImpl;
}

pub trait ContextWidget {
    type Widget: Widget;

    fn get_widgets(&self) -> &Tree<WidgetId, WidgetElement>;

    fn get_widget_id(&self) -> WidgetId;

    fn get_widget(&self) -> &Self::Widget;
}

pub trait ContextStatefulWidget: ContextWidget
where
    Self::Widget: WidgetState,
{
    fn get_state(&self) -> &<Self::Widget as WidgetState>::State;

    fn get_state_mut(&mut self) -> &mut <Self::Widget as WidgetState>::State;

    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut <Self::Widget as WidgetState>::State);
}

pub trait ContextWidgetMut: ContextWidget {
    fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: Data,
        F: Fn(&mut CallbackContext<Self::Widget>, &A) + 'static;
}

use std::any::TypeId;

use downcast_rs::{impl_downcast, Downcast};

use crate::manager::{context::AguiContext, event::WidgetEvent, Data};

mod context;

pub use context::*;

pub trait PluginImpl: std::fmt::Debug + Downcast {
    fn get_type_id(&self) -> TypeId;
    fn get_display_name(&self) -> String;

    fn on_before_update(&mut self, ctx: AguiContext);
    fn on_update(&mut self, ctx: AguiContext);
    fn on_layout(&mut self, ctx: AguiContext);
    fn on_events(&mut self, ctx: AguiContext, events: &[WidgetEvent]);
}

impl_downcast!(PluginImpl);

/// A plugin for the widget manager.
#[allow(unused_variables)]
pub trait WidgetManagerPlugin: std::fmt::Debug + Downcast {
    type State: Data + Default;

    /// Fired every time the widget manager is updated, before any widgets are updated.
    fn on_before_update(&self, ctx: &mut PluginContext, state: &mut Self::State) {}

    /// Fired every time the widget manager is updated, after all widgets are updated.
    fn on_update(&self, ctx: &mut PluginContext, state: &mut Self::State) {}

    /// Fired after widgets are updated, just after the layout is resolved.
    ///
    /// This may listen to changes, however it's fired following the layout being resolved, meaning
    /// it has up-to-date information on real widget size. This may listen and react to state, but if
    /// possible it should only modify state if absolutely necessary because any update notifications
    /// will cause the layout to be recalculated.
    fn on_layout(&self, ctx: &mut PluginContext, state: &mut Self::State) {}

    /// Allows the plugin to listen to widget tree events.
    fn on_events(&self, ctx: &mut PluginContext, state: &mut Self::State, events: &[WidgetEvent]) {}
}

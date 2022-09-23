use downcast_rs::Downcast;

use crate::{manager::widgets::events::WidgetEvent, unit::Data};

use super::PluginContext;

#[allow(unused_variables)]
pub trait PluginImpl: Downcast {
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

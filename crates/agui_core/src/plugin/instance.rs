use std::any::TypeId;

use downcast_rs::{impl_downcast, Downcast};

use crate::manager::{context::AguiContext, event::WidgetEvent};

/// A plugin for the widget manager.
pub trait PluginInstance: std::fmt::Debug + Downcast {
    fn get_type_id(&self) -> TypeId;
    fn get_display_name(&self) -> String;

    /// Fired every time the widget manager is updated, before any widgets are updated.
    fn on_before_update(&mut self, ctx: AguiContext);

    /// Fired every time the widget manager is updated, after all widgets are updated.
    fn on_update(&mut self, ctx: AguiContext);

    /// Fired after widgets are updated, just after the layout is resolved.
    ///
    /// This may listen to changes, however it's fired following the layout being resolved, meaning
    /// it has up-to-date information on real widget size. This may listen and react to state, but if
    /// possible it should only modify state if absolutely necessary because any update notifications
    /// will cause the layout to be recalculated.
    fn on_layout(&mut self, ctx: AguiContext);

    /// Allows the plugin to listen to widget tree events.
    fn on_events(&mut self, ctx: AguiContext, events: &[WidgetEvent]);
}

impl_downcast!(PluginInstance);

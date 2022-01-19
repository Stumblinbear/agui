use std::any::TypeId;

use downcast_rs::{impl_downcast, Downcast};

use crate::engine::event::WidgetEvent;

mod context;

pub use context::PluginContext;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct PluginId(TypeId);

impl PluginId {
    pub fn of<F>() -> Self
    where
        F: ?Sized + 'static,
    {
        Self(TypeId::of::<F>())
    }
}

/// A plugin for the widget system.
pub trait EnginePlugin: Downcast + Send + Sync {
    /// Fired every time the engine is updated, before any widgets are updated.
    fn pre_update(&self, ctx: &mut PluginContext);

    /// Fired in the same context as a widget `build()` context.
    ///
    /// Plugins utilizing this are essentially widgets that don't exist in the widget tree. They
    /// may have state, listen to state, etc, but do not contain children.
    fn on_update(&self, ctx: &mut PluginContext);

    /// Fired after widgets are updated, just after the layout is resolved.
    ///
    /// This may listen to changes, however it's fired following the layout being resolved, meaning
    /// it has up-to-date information on real widget size. This may listen and react to state, but if
    /// possible it should only modify state if absolutely necessary because any update notifications
    /// will cause the layout to be recalculated.
    fn post_update(&self, ctx: &mut PluginContext);

    /// Allows the plugin to listen to widget tree events.
    ///
    /// This may only react to changes. If it updates state, it will not actually cause changes until
    /// the next frame or update call, possibly causing flickering if used incorrectly.
    fn on_events(&self, ctx: &mut PluginContext, events: &[WidgetEvent]);
}

impl_downcast!(EnginePlugin);

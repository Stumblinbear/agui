use downcast_rs::Downcast;

use crate::{manager::event::WidgetEvent, unit::Data};

use super::{IntoPlugin, PluginContext, PluginElement, PluginImpl};

#[allow(unused_variables)]
pub trait StatefulPlugin: std::fmt::Debug + Downcast {
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

impl<P> PluginImpl for P
where
    P: StatefulPlugin,
{
    type State = P::State;

    fn on_before_update(&self, ctx: &mut PluginContext, state: &mut Self::State) {
        self.on_before_update(ctx, state);
    }

    fn on_update(&self, ctx: &mut PluginContext, state: &mut Self::State) {
        self.on_update(ctx, state);
    }

    fn on_layout(&self, ctx: &mut PluginContext, state: &mut Self::State) {
        self.on_layout(ctx, state);
    }

    fn on_events(&self, ctx: &mut PluginContext, state: &mut Self::State, events: &[WidgetEvent]) {
        self.on_events(ctx, state, events);
    }
}

impl<P, S> IntoPlugin for P
where
    P: StatefulPlugin<State = S>,
    S: Data + Default,
{
    fn into_plugin(self) -> super::BoxedPlugin {
        Box::new(PluginElement::new(self))
    }
}

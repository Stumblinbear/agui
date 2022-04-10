use agui_core::{
    callback::CallbackContext,
    engine::{event::WidgetEvent, Data, Engine},
    plugin::{EnginePlugin, PluginContext},
    prelude::BuildContext,
};

pub trait Event: Data {}

#[derive(Debug, Default)]
pub struct EventPlugin;

impl EnginePlugin for EventPlugin {
    type State = EventState;

    // Check if any changes occurred outside of the main engine loop.
    fn on_before_update(&self, ctx: &mut PluginContext, state: &mut EventState) {
        self.on_update(ctx, state);
    }

    fn on_update(&self, ctx: &mut PluginContext, state: &mut EventState) {}

    fn on_events(&self, _: &mut PluginContext, state: &mut EventState, events: &[WidgetEvent]) {
        for event in events {
            if let WidgetEvent::Destroyed { widget_id, .. } = event {
                // If the widget is listening to something, remove it from the respective listeners
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct EventState {}

impl EventState {}

pub trait EventPluginEngineExt {
    fn fire_event<E>(&mut self, event: E)
    where
        E: Event;
}

pub trait EventPluginContextExt<S>
where
    S: Data,
{
    fn listen_to<E, F>(&mut self, func: F)
    where
        E: Event,
        F: Fn(&mut CallbackContext<S>, &E) + 'static;

    fn fire_event<E>(&mut self, event: E)
    where
        E: Event;
}

impl EventPluginEngineExt for Engine {
    fn fire_event<E>(&mut self, event: E)
    where
        E: Event,
    {
        let mut plugin = self
            .get_plugin_mut::<EventPlugin>()
            .expect("event plugin not added");

        let state = plugin.get_state_mut();
    }
}

impl<'ctx, S> EventPluginContextExt<S> for BuildContext<'ctx, S>
where
    S: Data,
{
    fn listen_to<E, F>(&mut self, func: F)
    where
        E: Event,
        F: Fn(&mut CallbackContext<S>, &E) + 'static,
    {
        let mut plugin = self
            .get_plugin_mut::<EventPlugin>()
            .expect("event plugin not added");

        let state = plugin.get_state_mut();

        let callback_id = self.callback(func);
    }

    fn fire_event<E>(&mut self, event: E)
    where
        E: Event,
    {
        let mut plugin = self
            .get_plugin_mut::<EventPlugin>()
            .expect("event plugin not added");

        let state = plugin.get_state_mut();
    }
}

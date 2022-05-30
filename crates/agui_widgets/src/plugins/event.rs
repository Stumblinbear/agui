use std::{any::TypeId, collections::HashSet, rc::Rc};

use agui_core::{
    callback::{CallbackContext, CallbackId},
    manager::{context::Context, event::WidgetEvent, Data, WidgetManager},
    plugin::{PluginContext, StatefulPlugin},
    util::map::{TypeMap, TypeSet, WidgetMap},
    widget::{BuildContext, WidgetImpl},
};

#[derive(Debug, Default)]
pub struct EventPlugin;

impl StatefulPlugin for EventPlugin {
    type State = EventState;

    // Check if any changes occurred outside of the main loop.
    fn on_before_update(&self, ctx: &mut PluginContext, state: &mut Self::State) {
        self.on_update(ctx, state);
    }

    fn on_update(&self, ctx: &mut PluginContext, state: &mut Self::State) {
        for event in state.queue.drain(..) {
            let type_id = event.type_id();

            if let Some(callbacks) = state.callbacks.get(&type_id) {
                for callback_id in callbacks {
                    unsafe { ctx.call_unsafe(*callback_id, Rc::clone(&event)) }
                }
            }
        }
    }

    fn on_events(&self, _: &mut PluginContext, state: &mut Self::State, events: &[WidgetEvent]) {
        for event in events {
            if let WidgetEvent::Destroyed { widget_id, .. } = event {
                // If the widget is listening to something, remove it from the respective listeners
                if let Some(types) = state.listening.remove(widget_id) {
                    for type_id in types {
                        state
                            .callbacks
                            .get_mut(&type_id)
                            .unwrap()
                            .retain(|value| value.get_widget_id() != *widget_id);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct EventState {
    listening: WidgetMap<TypeSet>,
    callbacks: TypeMap<HashSet<CallbackId>>,

    queue: Vec<Rc<dyn Data>>,
}

impl EventState {
    fn listen_to<E>(&mut self, callback_id: CallbackId)
    where
        E: Data,
    {
        let type_id = TypeId::of::<E>();

        self.listening
            .entry(callback_id.get_widget_id())
            .or_insert_with(TypeSet::default)
            .insert(type_id);

        self.callbacks
            .entry(type_id)
            .or_insert_with(HashSet::default)
            .insert(callback_id);
    }

    fn fire_event<E>(&mut self, event: E)
    where
        E: Data,
    {
        let type_id = TypeId::of::<E>();

        if !self.callbacks.contains_key(&type_id) {
            return;
        }

        self.queue.push(Rc::new(event));
    }
}

pub trait EventPluginExt {
    fn fire_event<E>(&mut self, event: E)
    where
        E: Data;
}

pub trait EventPluginContextExt<W>
where
    W: WidgetImpl,
{
    fn listen_to<E, F>(&mut self, func: F)
    where
        E: Data,
        F: Fn(&mut CallbackContext<W>, &E) + 'static;

    fn fire_event<E>(&mut self, event: E)
    where
        E: Data;
}

impl EventPluginExt for WidgetManager {
    fn fire_event<E>(&mut self, event: E)
    where
        E: Data,
    {
        if let Some(plugin) = self.get_plugin_mut::<EventPlugin>() {
            plugin.get_state_mut().fire_event(event)
        }
    }
}

impl<'ctx, W> EventPluginContextExt<W> for BuildContext<'ctx, W>
where
    W: WidgetImpl,
{
    fn listen_to<E, F>(&mut self, func: F)
    where
        E: Data,
        F: Fn(&mut CallbackContext<W>, &E) + 'static,
    {
        let callback_id = self.callback(func).get_id().unwrap();

        if let Some(plugin) = self.get_plugin_mut::<EventPlugin>() {
            plugin.get_state_mut().listen_to::<E>(callback_id)
        }
    }

    fn fire_event<E>(&mut self, event: E)
    where
        E: Data,
    {
        if let Some(plugin) = self.get_plugin_mut::<EventPlugin>() {
            plugin.get_state_mut().fire_event(event)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use agui_core::{
        manager::{context::Context, query::WidgetQueryExt, WidgetManager},
        widget::{BuildContext, BuildResult, StatefulWidget},
    };
    use agui_primitives::Column;

    use crate::plugins::event::{EventPlugin, EventPluginExt};

    use super::EventPluginContextExt;

    #[derive(Clone, Debug, Default)]
    struct TestListener {}

    impl StatefulWidget for TestListener {
        type State = u32;

        fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
            ctx.listen_to::<u32, _>(|ctx, event| {
                ctx.set_state(|state| {
                    *state = *event;
                });
            });

            BuildResult::None
        }
    }

    #[test]
    pub fn tracks_listeners() {
        let mut manager = WidgetManager::with_root(TestListener::default());

        manager.add_plugin(EventPlugin::default());

        manager.update();

        let plugin = manager.get_plugin::<EventPlugin>().unwrap();
        let callbacks = plugin.get_state().callbacks.get(&TypeId::of::<u32>());
        let listening = &plugin.get_state().listening;

        assert!(callbacks.is_some(), "should have tracked the callbacks");
        assert!(!listening.is_empty(), "should have tracked the listener");
    }

    #[test]
    pub fn does_not_leak_memory() {
        let mut manager = WidgetManager::with_root(TestListener::default());

        manager.add_plugin(EventPlugin::default());

        manager.update();

        manager.set_root(TestListener::default());

        manager.update();

        let plugin = manager.get_plugin::<EventPlugin>().unwrap();
        let listening = &plugin.get_state().listening;
        let callbacks = &plugin.get_state().callbacks;

        assert!(listening.len() == 1, "only one widget should be tracked");
        assert!(callbacks.len() == 1, "only one callback should be tracked");
    }

    #[test]
    pub fn queue_events() {
        let mut manager = WidgetManager::with_root(TestListener::default());

        manager.add_plugin(EventPlugin::default());

        manager.update();

        manager.fire_event(7_u32);

        let plugin = manager.get_plugin::<EventPlugin>().unwrap();
        let queue = &plugin.get_state().queue;

        assert!(!queue.is_empty(), "should have queued the event");

        manager.update();

        let plugin = manager.get_plugin::<EventPlugin>().unwrap();
        let queue = &plugin.get_state().queue;

        assert!(
            queue.is_empty(),
            "should have remove the event from the queue"
        );
    }

    #[test]
    pub fn listening_to_events() {
        let mut manager = WidgetManager::with_root(TestListener::default());

        manager.add_plugin(EventPlugin::default());

        manager.update();

        assert_eq!(
            *manager
                .query()
                .by_type::<TestListener>()
                .next()
                .unwrap()
                .get_state(),
            0,
            "initial state should be zero"
        );

        manager.fire_event(7_u32);

        manager.update();

        assert_eq!(
            *manager
                .query()
                .by_type::<TestListener>()
                .next()
                .unwrap()
                .get_state(),
            7,
            "state should have updated to the event's value"
        );
    }

    #[test]
    pub fn multiple_widgets_listening() {
        let mut manager = WidgetManager::with_root(Column {
            children: vec![
                TestListener::default().into(),
                TestListener::default().into(),
            ],
            ..Default::default()
        });

        manager.add_plugin(EventPlugin::default());

        manager.update();

        for widget in manager.query().by_type::<TestListener>() {
            assert_eq!(
                *widget.get_state(),
                0,
                "state should have updated to the event's value"
            );
        }

        manager.fire_event(7_u32);

        manager.update();

        for widget in manager.query().by_type::<TestListener>() {
            assert_eq!(
                *widget.get_state(),
                7,
                "state should have updated to the event's value"
            );
        }
    }
}

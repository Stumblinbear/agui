use std::{any::TypeId, collections::HashSet};

use agui_core::{
    callback::{CallbackContext, CallbackId},
    manager::{events::WidgetEvent, WidgetManager},
    plugin::{PluginContext, StatefulPlugin},
    unit::Data,
    util::map::{TypeMap, TypeSet, WidgetMap},
    widget::{ContextPlugins, ContextWidgetMut, Widget, WidgetState},
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
                unsafe {
                    ctx.call_many_unsafe(&callbacks.iter().copied().collect::<Vec<_>>(), event);
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

#[derive(Default)]
pub struct EventState {
    listening: WidgetMap<TypeSet>,
    callbacks: TypeMap<HashSet<CallbackId>>,

    queue: Vec<Box<dyn Data>>,
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

        self.queue.push(Box::new(event));
    }
}

pub trait EventPluginExt {
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

pub trait ContextEventPluginExt<W>
where
    W: Widget + WidgetState,
{
    fn listen_to<E, F>(&mut self, func: F)
    where
        E: Data,
        F: Fn(&mut CallbackContext<W>, &E) + 'static;

    fn fire_event<E>(&mut self, event: E)
    where
        E: Data;
}

impl<C, W> ContextEventPluginExt<W> for C
where
    C: ContextPlugins + ContextWidgetMut<Widget = W>,
    W: Widget + WidgetState,
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
    use std::{any::TypeId, cell::RefCell};

    use agui_core::{
        manager::WidgetManager,
        widget::{BuildContext, BuildResult, WidgetState, WidgetView},
    };
    use agui_macros::StatefulWidget;
    use agui_primitives::Column;

    use crate::plugins::event::{EventPlugin, EventPluginExt};

    use super::ContextEventPluginExt;

    thread_local! {
        pub static STATE: RefCell<Vec<u32>> = RefCell::default();
    }

    #[derive(StatefulWidget, Clone, Debug, Default, PartialEq)]
    struct TestListener {}

    impl WidgetState for TestListener {
        type State = u32;

        fn create_state(&self) -> Self::State {
            0
        }
    }

    impl WidgetView for TestListener {
        fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
            ctx.listen_to::<u32, _>(|_, event| {
                STATE.with(|f| {
                    f.borrow_mut().push(*event);
                });
            });

            BuildResult::empty()
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

        manager.fire_event(7_u32);

        manager.update();

        STATE.with(|f| {
            assert_eq!(f.borrow()[0], 7, "widget should have received event");
        });
    }

    #[test]
    pub fn multiple_widgets_listening() {
        let mut manager = WidgetManager::with_root(Column {
            children: [
                TestListener::default().into(),
                TestListener::default().into(),
            ]
            .into(),
            ..Default::default()
        });

        manager.add_plugin(EventPlugin::default());

        manager.update();

        manager.fire_event(7_u32);

        manager.update();

        STATE.with(|f| {
            assert_eq!(f.borrow()[0], 7, "first widget should have received event");
            assert_eq!(f.borrow()[1], 7, "second widget should have received event");
        });
    }
}

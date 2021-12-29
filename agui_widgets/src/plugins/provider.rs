use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use agui_core::{
    context::{Notify, Value, WidgetContext},
    event::WidgetEvent,
    plugin::WidgetPlugin,
    widget::WidgetId,
};
use parking_lot::Mutex;

#[derive(Default)]
pub struct Provider {
    providers: Arc<Mutex<HashMap<TypeId, HashSet<WidgetId>>>>,
    widgets: Arc<Mutex<HashMap<WidgetId, HashSet<TypeId>>>>,
}

impl WidgetPlugin for Provider {
    fn on_update(&self, _ctx: &WidgetContext) {
        
    }

    fn on_events(&self, _ctx: &WidgetContext, events: &[WidgetEvent]) {
        for event in events {
            if let WidgetEvent::Destroyed { widget_id, .. } = event {
                if let Some(providing) = self.widgets.lock().remove(widget_id) {
                    let mut providers = self.providers.lock();

                    for type_id in providing {
                        let widgets = providers.get_mut(&type_id).expect("provider map broken");

                        widgets.remove(widget_id);
                    }
                }
            }
        }
    }
}

impl Provider {
    /// Creates a local state and makes it available to any child widget.
    pub fn provide_value<V>(ctx: &WidgetContext, value: V) -> Notify<V>
    where
        V: Value,
    {
        // Make the state available before setting
        Self::provide_state::<V>(ctx);

        ctx.set_state(value)
    }

    /// Makes some local widget state available to any child widget.
    pub fn provide_state<V>(ctx: &WidgetContext)
    where
        V: Value,
    {
        let plugin = ctx.get_or_init_plugin::<Provider>();

        let mut providers = plugin.providers.lock();

        let type_id = TypeId::of::<V>();

        providers
            .entry(type_id)
            .or_insert_with(HashSet::default)
            .insert(ctx.get_self());

        let mut widgets = plugin.widgets.lock();

        let widget_id = ctx.get_self();

        widgets
            .entry(widget_id)
            .or_insert_with(HashSet::new)
            .insert(type_id);
    }

    /// Consume the state of the first parent that is providing it.
    pub fn of<S>(ctx: &WidgetContext) -> Option<Notify<S>>
    where
        S: Value,
    {
        if let Some(plugin) = ctx.get_plugin::<Provider>() {
            let providers = plugin.providers.lock();

            if let Some(providers) = providers.get(&TypeId::of::<S>()) {
                for parent_id in ctx.get_tree().iter_parents(ctx.get_self()) {
                    if providers.contains(&parent_id) {
                        return Some(
                            ctx.get_state_for(parent_id, || panic!("provider state broken")),
                        );
                    }
                }
            }
        }

        // Fall back to global state
        if let Some(state) = ctx.get_global::<S>() {
            return Some(state);
        }

        None
    }
}

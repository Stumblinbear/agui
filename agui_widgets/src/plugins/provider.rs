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
    fn on_update(&self, _ctx: &WidgetContext) {}

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

pub trait ProviderExt<'ui> {
    fn provide(&self, ctx: &WidgetContext);
}

impl<'ui, V> ProviderExt<'ui> for Notify<V>
where
    V: Value,
{
    /// Makes some local widget state available to any child widget.
    fn provide(&self, ctx: &WidgetContext) {
        let plugin = ctx.get_plugin_or::<Provider, _>(Provider::default);

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
}

pub trait ConsumerExt<'ui> {
    fn consume<V>(&self) -> Option<Notify<V>>
    where
        V: Value;
}

impl<'ui> ConsumerExt<'ui> for WidgetContext<'ui> {
    /// Makes some local widget state available to any child widget.
    fn consume<V>(&self) -> Option<Notify<V>>
    where
        V: Value,
    {
        if let Some(plugin) = self.get_plugin::<Provider>() {
            let providers = plugin.providers.lock();

            if let Some(providers) = providers.get(&TypeId::of::<V>()) {
                for parent_id in self.get_tree().iter_parents(self.get_self()) {
                    if providers.contains(&parent_id) {
                        return Some(
                            self.get_state_for(parent_id, || panic!("provider state broken")),
                        );
                    }
                }
            }
        }

        // Fall back to global state
        if let Some(state) = self.get_global::<V>() {
            return Some(state);
        }

        None
    }
}

use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
};

use agui_core::{
    context::{ListenerId, NotifiableValue, Notify, WidgetContext},
    event::WidgetEvent,
    plugin::WidgetPlugin,
    widget::WidgetId,
};

#[derive(Debug, Default)]
struct ProviderPluginState {
    providers: HashMap<TypeId, HashSet<WidgetId>>,
    widgets: HashMap<WidgetId, HashSet<TypeId>>,
}

#[derive(Default)]
pub struct ProviderPlugin;

impl WidgetPlugin for ProviderPlugin {
    fn pre_update(&self, _ctx: &mut WidgetContext) {}

    fn on_update(&self, _ctx: &mut WidgetContext) {}

    fn post_update(&self, _ctx: &mut WidgetContext) {}

    fn on_events(&self, ctx: &mut WidgetContext, events: &[WidgetEvent]) {
        let plugin = ctx.init_global(ProviderPluginState::default);

        let mut plugin = plugin.write();

        for event in events {
            if let WidgetEvent::Destroyed { widget_id, .. } = event {
                if let Some(providing) = plugin.widgets.remove(widget_id) {
                    for type_id in providing {
                        let widgets = plugin
                            .providers
                            .get_mut(&type_id)
                            .expect("provider map broken");

                        widgets.remove(widget_id);
                    }
                }
            }
        }
    }
}

pub trait ProviderExt<'ui> {
    fn provide(&self, ctx: &mut WidgetContext);
}

impl<'ui, V> ProviderExt<'ui> for Notify<V>
where
    V: NotifiableValue,
{
    /// Makes some local widget state available to any child widget.
    fn provide(&self, ctx: &mut WidgetContext) {
        let plugin = ctx.init_global(ProviderPluginState::default);

        let mut plugin = plugin.write();

        let type_id = TypeId::of::<V>();

        let widget_id = ctx
            .get_self()
            .widget_id()
            .expect("cannot provide state outside of a widget context");

        plugin
            .providers
            .entry(type_id)
            .or_insert_with(HashSet::default)
            .insert(widget_id);

        plugin
            .widgets
            .entry(widget_id)
            .or_insert_with(HashSet::new)
            .insert(type_id);
    }
}

pub trait ConsumerExt<'ui> {
    fn consume<V>(&mut self) -> Option<Notify<V>>
    where
        V: NotifiableValue;
}

impl<'ui> ConsumerExt<'ui> for WidgetContext<'ui> {
    /// Makes some local widget state available to any child widget.
    fn consume<V>(&mut self) -> Option<Notify<V>>
    where
        V: NotifiableValue,
    {
        let plugin = self.init_global(ProviderPluginState::default);

        let plugin = plugin.write();

        if let Some(providers) = plugin.providers.get(&TypeId::of::<V>()) {
            let widget_id = self
                .get_self()
                .widget_id()
                .expect("cannot provide state outside of a widget context");

            // If the widget calling this is also providing the state, return that.
            if providers.contains(&widget_id) {
                return Some(
                    self.use_state_of(self.get_self(), || panic!("provider state broken")),
                );
            }

            for parent_id in self.get_tree().iter_parents(widget_id) {
                if providers.contains(&parent_id) {
                    return Some(self.use_state_of(ListenerId::Widget(parent_id), || {
                        panic!("provider state broken")
                    }));
                }
            }
        }

        // Fall back to global state
        if let Some(state) = self.try_use_global::<V>() {
            return Some(state);
        }

        None
    }
}

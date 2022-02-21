use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
};

use agui_core::{
    engine::event::WidgetEvent,
    plugin::{EnginePlugin, PluginContext},
    state::{State, StateValue},
    widget::{BuildContext, WidgetContext, WidgetId},
};

#[derive(Debug, Default, Clone)]
struct ProviderPluginState {
    providers: HashMap<TypeId, HashSet<WidgetId>>,
    widgets: HashMap<WidgetId, HashSet<TypeId>>,
}

#[derive(Default)]
pub struct ProviderPlugin;

impl EnginePlugin for ProviderPlugin {
    fn on_update(&self, _ctx: &mut PluginContext) {}

    fn on_build(&self, _ctx: &mut PluginContext) {}

    fn on_layout(&self, _ctx: &mut PluginContext) {}

    fn on_events(&self, ctx: &mut PluginContext, events: &[WidgetEvent]) {
        let plugin = ctx.init_global(ProviderPluginState::default);

        let mut removed_widgets = Vec::new();

        for event in events {
            if let WidgetEvent::Destroyed { widget_id, .. } = event {
                if plugin.widgets.contains_key(widget_id) {
                    removed_widgets.push(widget_id);
                }
            }
        }

        if !removed_widgets.is_empty() {
            let mut plugin = plugin.write();

            for widget_id in removed_widgets {
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

pub trait ProviderExt {
    fn provide(&self, ctx: &mut BuildContext);
}

impl<'ui, V> ProviderExt for State<V>
where
    V: StateValue,
{
    /// Makes some local widget state available to any child widget.
    fn provide(&self, ctx: &mut BuildContext) {
        let mut plugin = ctx.init_global(ProviderPluginState::default).write();

        let type_id = TypeId::of::<V>();

        let widget_id = ctx.get_widget();

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

pub trait ConsumerExt {
    fn consume<V>(&mut self) -> Option<State<V>>
    where
        V: StateValue;
}

impl<'ui, 'ctx> ConsumerExt for BuildContext<'ui, 'ctx> {
    /// Makes some local widget state available to any child widget.
    fn consume<V>(&mut self) -> Option<State<V>>
    where
        V: StateValue,
    {
        let plugin = self.init_global(ProviderPluginState::default);

        if let Some(providers) = plugin.providers.get(&TypeId::of::<V>()) {
            let widget_id = self.get_widget();

            // If the widget calling this is also providing the state, return that.
            if providers.contains(&widget_id) {
                return Some(self.use_state(|| panic!("provider state broken")));
            }

            for parent_id in self.get_tree().iter_parents(widget_id) {
                if providers.contains(&parent_id) {
                    return Some(
                        self.use_state_from(parent_id, || panic!("provider state broken")),
                    );
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

impl<'ui, 'ctx> ConsumerExt for WidgetContext<'ui, 'ctx> {
    /// Makes some local widget state available to any child widget.
    fn consume<V>(&mut self) -> Option<State<V>>
    where
        V: StateValue,
    {
        let plugin = self.init_global(ProviderPluginState::default);

        if let Some(providers) = plugin.providers.get(&TypeId::of::<V>()) {
            let widget_id = self.get_widget();

            // If the widget calling this is also providing the state, return that.
            if providers.contains(&widget_id) {
                return Some(self.use_state(|| panic!("provider state broken")));
            }

            for parent_id in self.get_tree().iter_parents(widget_id) {
                if providers.contains(&parent_id) {
                    return Some(
                        self.use_state_from(parent_id, || panic!("provider state broken")),
                    );
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

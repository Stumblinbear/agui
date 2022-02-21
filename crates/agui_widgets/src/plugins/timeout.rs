use std::{
    collections::HashMap,
    ops::Add,
    time::{Duration, Instant},
};

use agui_core::{
    engine::event::WidgetEvent,
    plugin::{EnginePlugin, PluginContext},
    state::ListenerId,
    widget::{BuildContext, WidgetContext, WidgetId},
};

#[derive(Debug, Default, Clone)]
struct TimeoutPluginState {
    widgets: HashMap<WidgetId, HashMap<ListenerId, Instant>>,
}

impl TimeoutPluginState {
    pub fn create_timeout(&mut self, listener_id: ListenerId, duration: Duration) {
        let entry = self
            .widgets
            .entry(
                listener_id
                    .widget_id()
                    .expect("cannot use timers outside of a widget context"),
            )
            .or_insert_with(HashMap::default)
            .entry(listener_id);

        let target_instant = Instant::now().add(duration);

        entry
            .and_modify(|instant| {
                *instant = (*instant).min(target_instant);
            })
            .or_insert(target_instant);
    }
}

#[derive(Default)]
pub struct TimeoutPlugin;

impl EnginePlugin for TimeoutPlugin {
    fn on_update(&self, ctx: &mut PluginContext) {
        let plugin = ctx.init_global(TimeoutPluginState::default);

        let now = Instant::now();

        let mut updated_timeouts = HashMap::new();

        for (widget_id, timeouts) in &plugin.widgets {
            let mut updated = Vec::new();

            for (listener_id, instant) in timeouts.iter() {
                // Loop until we find the first timeout that hasn't been met
                if now > *instant {
                    ctx.mark_dirty(*listener_id);

                    updated.push(*listener_id);
                }
            }

            if !updated.is_empty() {
                updated_timeouts.insert(*widget_id, updated);
            }
        }

        if !updated_timeouts.is_empty() {
            let mut plugin = plugin.write();

            for (widget_id, updated) in updated_timeouts {
                for pair in updated {
                    plugin.widgets.get_mut(&widget_id).unwrap().remove(&pair);
                }
            }
        }
    }

    fn on_build(&self, _ctx: &mut PluginContext) {}

    fn on_layout(&self, _ctx: &mut PluginContext) {}

    fn on_events(&self, ctx: &mut PluginContext, events: &[WidgetEvent]) {
        let plugin = ctx.init_global(TimeoutPluginState::default);

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
                plugin.widgets.remove(widget_id);
            }
        }
    }
}

pub trait TimeoutExt {
    fn use_timeout(&mut self, duration: Duration);
}

impl<'ui, 'ctx> TimeoutExt for BuildContext<'ui, 'ctx> {
    /// Marks the caller for updating when `duration` elapses.
    fn use_timeout(&mut self, duration: Duration) {
        self.init_global(TimeoutPluginState::default)
            .write()
            .create_timeout(self.get_listener(), duration);
    }
}

impl<'ui, 'ctx> TimeoutExt for WidgetContext<'ui, 'ctx> {
    /// Marks the caller for updating when `duration` elapses.
    fn use_timeout(&mut self, duration: Duration) {
        self.init_global(TimeoutPluginState::default)
            .write()
            .create_timeout(self.get_listener(), duration);
    }
}

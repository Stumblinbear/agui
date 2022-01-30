use std::{
    collections::{BTreeSet, HashMap},
    ops::Add,
    time::{Duration, Instant},
};

use agui_core::{
    computed::ComputedContext,
    engine::event::WidgetEvent,
    notifiable::ListenerId,
    plugin::{EnginePlugin, PluginContext},
    widget::{WidgetContext, WidgetId},
};

#[derive(Debug, Default)]
struct TimeoutPluginState {
    widgets: HashMap<WidgetId, BTreeSet<(Instant, ListenerId)>>,
}

impl TimeoutPluginState {
    pub fn create_timeout(&mut self, listener_id: ListenerId, duration: Duration) {
        self.widgets
            .entry(
                listener_id
                    .widget_id()
                    .expect("cannot use timers outside of a widget context"),
            )
            .or_insert_with(BTreeSet::default)
            .insert((Instant::now().add(duration), listener_id));
    }
}

#[derive(Default)]
pub struct TimeoutPlugin;

impl EnginePlugin for TimeoutPlugin {
    fn on_update(&self, ctx: &mut PluginContext) {
        let plugin = ctx.init_global(TimeoutPluginState::default);

        let mut plugin = plugin.write();

        let now = Instant::now();

        for timeouts in plugin.widgets.values_mut() {
            let mut updated = Vec::new();

            for pair in timeouts.iter() {
                // Loop until we find the first timeout that hasn't been met
                if now > pair.0 {
                    ctx.mark_dirty(pair.1);

                    updated.push(*pair);
                } else {
                    break;
                }
            }

            for pair in updated {
                timeouts.remove(&pair);
            }
        }
    }

    fn on_build(&self, _ctx: &mut PluginContext) {}

    fn on_layout(&self, _ctx: &mut PluginContext) {}

    fn on_events(&self, ctx: &mut PluginContext, events: &[WidgetEvent]) {
        let plugin = ctx.init_global(TimeoutPluginState::default);

        let mut plugin = plugin.write();

        for event in events {
            if let WidgetEvent::Destroyed { widget_id, .. } = event {
                plugin.widgets.remove(widget_id);
            }
        }
    }
}

pub trait TimeoutExt {
    fn use_timeout(&mut self, duration: Duration);
}

impl<'ui, 'ctx> TimeoutExt for WidgetContext<'ui, 'ctx> {
    /// Marks the caller for updating when `duration` elapses.
    fn use_timeout(&mut self, duration: Duration) {
        self.init_global(TimeoutPluginState::default)
            .write()
            .create_timeout(self.get_listener(), duration);
    }
}

impl<'ui, 'ctx> TimeoutExt for ComputedContext<'ui, 'ctx> {
    /// Marks the caller for updating when `duration` elapses.
    fn use_timeout(&mut self, duration: Duration) {
        self.init_global(TimeoutPluginState::default)
            .write()
            .create_timeout(self.get_listener(), duration);
    }
}

use std::{
    collections::{BTreeSet, HashMap},
    ops::Add,
    time::{Duration, Instant},
};

use agui_core::{
    engine::event::WidgetEvent,
    notifiable::ListenerId,
    plugin::{EnginePlugin, PluginContext},
    widget::{BuildContext, WidgetContext, WidgetId},
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

        let now = Instant::now();

        let mut updated_timeouts = HashMap::new();

        {
            let plugin = plugin.read();

            for (widget_id, timeouts) in &plugin.widgets {
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

                if !updated.is_empty() {
                    updated_timeouts.insert(*widget_id, updated);
                }
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

        for event in events {
            if let WidgetEvent::Destroyed { widget_id, .. } = event {
                plugin.write().widgets.remove(widget_id);
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

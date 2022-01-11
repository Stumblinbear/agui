use std::{
    collections::{BTreeSet, HashMap},
    ops::Add,
    sync::Arc,
    time::{Duration, Instant},
};

use agui_core::{
    context::{ListenerId, WidgetContext},
    event::WidgetEvent,
    plugin::WidgetPlugin,
    widget::WidgetId,
};
use parking_lot::Mutex;

pub struct TimerPlugin {
    widgets: Arc<Mutex<HashMap<WidgetId, BTreeSet<(Instant, ListenerId)>>>>,
}

impl Default for TimerPlugin {
    fn default() -> Self {
        Self {
            widgets: Arc::default(),
        }
    }
}

impl TimerPlugin {
    pub fn create_timeout(&self, listener_id: ListenerId, duration: Duration) {
        self.widgets
            .lock()
            .entry(
                listener_id
                    .widget_id()
                    .expect("cannot use timers outside of a widget context"),
            )
            .or_insert_with(BTreeSet::default)
            .insert((Instant::now().add(duration), listener_id));
    }
}

impl WidgetPlugin for TimerPlugin {
    fn pre_update(&self, ctx: &WidgetContext) {
        let now = Instant::now();

        let mut widgets = self.widgets.lock();

        for timeouts in widgets.values_mut() {
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

    fn on_update(&self, _ctx: &WidgetContext) {}

    fn post_update(&self, _ctx: &WidgetContext) {}

    fn on_events(&self, _ctx: &WidgetContext, events: &[WidgetEvent]) {
        let mut widgets = self.widgets.lock();

        for event in events {
            if let WidgetEvent::Destroyed { widget_id, .. } = event {
                widgets.remove(widget_id);
            }
        }
    }
}

pub trait TimerExt<'ui> {
    fn use_timeout(&self, duration: Duration);
}

impl<'ui> TimerExt<'ui> for WidgetContext<'ui> {
    /// Marks the caller for updating when `duration` elapses.
    fn use_timeout(&self, duration: Duration) {
        self.init_plugin(TimerPlugin::default)
            .create_timeout(self.get_self(), duration);
    }
}

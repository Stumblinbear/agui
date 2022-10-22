use std::{
    collections::HashMap,
    ops::Add,
    time::{Duration, Instant},
};

use agui_core::{
    callback::{CallbackContext, CallbackId},
    manager::events::WidgetEvent,
    plugin::{PluginContext, StatefulPlugin},
    widget::{ContextPlugins, ContextWidgetMut, Widget, WidgetId, WidgetState},
};

#[derive(Debug, Default)]
pub struct TimeoutPlugin {}

impl StatefulPlugin for TimeoutPlugin {
    type State = TimeoutPluginState;

    /// Check if any timeouts have completed before the next update.
    fn on_before_update(&self, ctx: &mut PluginContext, state: &mut Self::State) {
        self.on_update(ctx, state);
    }

    fn on_update(&self, ctx: &mut PluginContext, state: &mut Self::State) {
        let now = Instant::now();

        let mut updated_timeouts = HashMap::new();

        for (widget_id, timeouts) in &state.widgets {
            let mut updated = Vec::new();

            // for (callback_id, instant) in timeouts.drain_filter(|k, instant| now > *instant) {
            //     unsafe { ctx.notify_unsafe(callback_id, Rc::clone(&self.dummy) as _) };
            // }

            for (callback_id, instant) in timeouts.iter() {
                if now > *instant {
                    tracing::debug!(
                        callback = format!("{:?}", callback_id).as_str(),
                        "timeout expired"
                    );

                    unsafe {
                        ctx.call_unsafe(*callback_id, Box::new(()));
                    };

                    updated.push(*callback_id);
                }
            }

            if !updated.is_empty() {
                updated_timeouts.insert(*widget_id, updated);
            }
        }

        if !updated_timeouts.is_empty() {
            for (widget_id, updated) in updated_timeouts {
                for pair in updated {
                    state.widgets.get_mut(&widget_id).unwrap().remove(&pair);
                }
            }
        }
    }

    fn on_events(&self, _: &mut PluginContext, state: &mut Self::State, events: &[WidgetEvent]) {
        for event in events {
            if let WidgetEvent::Destroyed { widget_id, .. } = event {
                state.widgets.remove(widget_id);
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TimeoutPluginState {
    widgets: HashMap<WidgetId, HashMap<CallbackId, Instant>>,
}

pub trait ContextTimeoutPluginExt<W>
where
    W: Widget + WidgetState,
{
    fn set_timeout<F>(&mut self, duration: Duration, func: F)
    where
        F: Fn(&mut CallbackContext<W>, &()) + 'static;
}

impl<C, W> ContextTimeoutPluginExt<W> for C
where
    C: ContextPlugins + ContextWidgetMut<Widget = W>,
    W: Widget + WidgetState,
{
    /// Marks the caller for updating when `duration` elapses.
    fn set_timeout<F>(&mut self, duration: Duration, func: F)
    where
        F: Fn(&mut CallbackContext<W>, &()) + 'static,
    {
        let callback_id = self.callback(func).get_id().unwrap();

        if let Some(plugin) = self.get_plugin_mut::<TimeoutPlugin>() {
            let state = plugin.get_state_mut();

            state
                .widgets
                .entry(callback_id.get_widget_id())
                .or_insert_with(HashMap::default)
                .insert(callback_id, Instant::now().add(duration));
        }
    }
}

use std::{ops::Deref, time::Instant};

use agui_core::{context::WidgetContext, event::WidgetEvent, plugin::WidgetPlugin};

pub struct TickPlugin {
    pub ticks_per_second: f32,
}

impl Default for TickPlugin {
    fn default() -> Self {
        Self {
            ticks_per_second: 10.0,
        }
    }
}

impl WidgetPlugin for TickPlugin {
    fn pre_update(&self, ctx: &WidgetContext) {
        let ticks = ctx.init_global(Tick::default);

        let secs_since_start = ticks.read().elapsed().as_secs_f32();

        let next_tick = (secs_since_start / (1.0 / self.ticks_per_second)).floor() as usize;

        // If the current ticks that should have passed is not equal to the current tick, update it
        if next_tick != ticks.read().tick {
            let mut tick = ticks.write();

            tick.tick = next_tick;
        }
    }

    fn on_update(&self, _ctx: &WidgetContext) {}

    fn post_update(&self, _ctx: &WidgetContext) {}

    fn on_events(&self, _ctx: &WidgetContext, _events: &[WidgetEvent]) {}
}

/// UI ticks are a less granular version of timer. It only updates approximately 4 times per second.
pub struct Tick {
    tick: usize,
    time: Instant,
}

impl Default for Tick {
    fn default() -> Self {
        Self {
            tick: 0,
            time: Instant::now(),
        }
    }
}

impl Deref for Tick {
    type Target = Instant;

    fn deref(&self) -> &Self::Target {
        &self.time
    }
}

impl Tick {
    pub fn tick(&self) -> usize {
        self.tick
    }

    pub fn now(&self) -> Instant {
        self.time
    }
}

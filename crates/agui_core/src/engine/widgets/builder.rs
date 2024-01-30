use std::{collections::VecDeque, sync::mpsc};

use agui_sync::notify;
use rustc_hash::FxHashSet;

use crate::{
    callback::CallbackQueue,
    engine::{
        elements::{
            scheduler::ElementSchedulerStrategy, strategies::InflateStrategy, tree::ElementTree,
        },
        widgets::WidgetManager,
        Dirty,
    },
};

pub struct WidgetManagerBuilder<Strat, Sched> {
    inflate_strategy: Strat,
    scheduler: Sched,

    notifier: Option<notify::Flag>,
}

impl<Strat, Sched> Default for WidgetManagerBuilder<Strat, Sched>
where
    Strat: InflateStrategy + Default,
    Sched: ElementSchedulerStrategy + Default,
{
    fn default() -> Self {
        Self {
            inflate_strategy: Strat::default(),
            scheduler: Sched::default(),

            notifier: None,
        }
    }
}

impl<Sched> WidgetManagerBuilder<(), Sched> {
    pub fn with_element_binding<Strat>(
        self,
        inflate_strategy: Strat,
    ) -> WidgetManagerBuilder<Strat, Sched>
    where
        Strat: InflateStrategy,
    {
        WidgetManagerBuilder {
            inflate_strategy,
            scheduler: self.scheduler,

            notifier: self.notifier,
        }
    }
}

impl<Strat> WidgetManagerBuilder<Strat, ()> {
    pub fn with_scheduler<Sched>(self, scheduler: Sched) -> WidgetManagerBuilder<Strat, Sched>
    where
        Sched: ElementSchedulerStrategy,
    {
        WidgetManagerBuilder {
            inflate_strategy: self.inflate_strategy,
            scheduler,

            notifier: self.notifier,
        }
    }
}

impl<Strat, Sched> WidgetManagerBuilder<Strat, Sched> {
    pub fn with_notifier(self, notifier: notify::Flag) -> WidgetManagerBuilder<Strat, Sched> {
        WidgetManagerBuilder {
            inflate_strategy: self.inflate_strategy,
            scheduler: self.scheduler,

            notifier: Some(notifier),
        }
    }
}

impl<Strat, Sched> WidgetManagerBuilder<Strat, Sched>
where
    Strat: InflateStrategy,
    Sched: ElementSchedulerStrategy,
{
    pub fn build(self) -> WidgetManager<Strat, Sched> {
        let notifier = self.notifier.unwrap_or_default();

        let (callback_tx, callback_rx) = mpsc::channel();

        WidgetManager {
            inflate_strategy: self.inflate_strategy,
            scheduler: self.scheduler,

            tree: ElementTree::default(),

            needs_build: Dirty::new(notifier.clone()),

            callback_rx,
            callback_queue: CallbackQueue::new(callback_tx, notifier),

            rebuild_queue: VecDeque::default(),
            forgotten_elements: FxHashSet::default(),
        }
    }
}

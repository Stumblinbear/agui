use std::{collections::VecDeque, sync::mpsc};

use rustc_hash::FxHashSet;
use tracing::field;

use crate::{
    callback::{CallbackQueue, InvokeCallback},
    element::{Element, ElementBuildContext, ElementCallbackContext, ElementId},
    engine::{
        elements::{
            context::ElementTreeContext,
            errors::{InflateError, SpawnElementError},
            scheduler::ElementSchedulerStrategy,
            strategies::InflateStrategy,
            tree::ElementTree,
        },
        Dirty,
    },
    widget::Widget,
};

mod builder;

pub use builder::*;

pub struct WidgetManager<Strat = (), Sched = ()> {
    inflate_strategy: Strat,
    scheduler: Sched,

    tree: ElementTree,

    needs_build: Dirty<ElementId>,

    callback_rx: mpsc::Receiver<InvokeCallback>,
    callback_queue: CallbackQueue,

    rebuild_queue: VecDeque<ElementId>,
    forgotten_elements: FxHashSet<ElementId>,
}

// impl Default for WidgetManager<(), ()> {
//     fn default() -> Self {
//         WidgetManagerBuilder::default().build()
//     }
// }

// impl WidgetManager<(), ()> {
//     pub fn builder() -> WidgetManagerBuilder<(), ()> {
//         WidgetManagerBuilder::default()
//     }
// }

// impl WidgetManager<(), ()> {
//     pub fn default_with_root(root: impl IntoWidget) -> Self {
//         WidgetManagerBuilder::default().build().with_root(root)
//     }
// }

// impl<Strat, Sched> WidgetManager<Strat, Sched> {
//     /// Get the element tree.
//     pub fn tree(&self) -> &ElementTree {
//         &self.tree
//     }

//     /// Get the root element.
//     pub fn root(&self) -> Option<ElementId> {
//         self.tree.root()
//     }

//     /// Check if an element exists in the tree.
//     pub fn contains(&self, element_id: ElementId) -> bool {
//         self.tree.contains(element_id)
//     }

//     pub fn callback_queue(&self) -> &CallbackQueue {
//         &self.callback_queue
//     }

//     /// Mark an element as dirty, causing it to be rebuilt on the next update.
//     pub fn mark_needs_build(&mut self, element_id: ElementId) {
//         tracing::trace!(?element_id, "element needs build");

//         self.needs_build.insert(element_id);
//     }
// }

#[cfg(test)]
impl WidgetManager<(), ()> {
    pub fn default_with_root(
        root: impl crate::widget::IntoWidget,
    ) -> WidgetManager<crate::engine::elements::strategies::tests::MockInflateStrategy, ()> {
        let mut manager = WidgetManagerBuilder::default().build();

        manager
            .inflate_root(root.into_widget())
            .expect("failed to inflate root");

        manager
    }
}

impl<Strat, Sched> WidgetManager<Strat, Sched>
where
    Strat: InflateStrategy,
    Sched: ElementSchedulerStrategy,
{
    /// Get the root element.
    pub fn inflate_root(&mut self, root: Widget) -> Result<ElementId, InflateError> {
        let is_first_root = self.tree.root().is_none();

        struct InflateRootStrategy<'ctx, Sched> {
            is_first_root: bool,

            scheduler: &'ctx mut Sched,

            callback_queue: &'ctx CallbackQueue,
            needs_build: &'ctx mut Dirty<ElementId>,

            forgotten_elements: &'ctx mut FxHashSet<ElementId>,
        }

        impl<Sched> InflateStrategy for InflateRootStrategy<'_, Sched>
        where
            Sched: ElementSchedulerStrategy,
        {
            fn on_spawned(&mut self, parent_id: Option<ElementId>, id: ElementId) {}

            fn on_updated(&mut self, id: ElementId) {
                if self.is_first_root {
                    tracing::error!(
                        "elements should never be updated while inflating the first root widget"
                    );
                }
            }

            fn on_forgotten(&mut self, id: ElementId) {
                if self.is_first_root {
                    tracing::error!(
                        "elements should never forgotten while inflating the first root widget"
                    );
                }

                self.forgotten_elements.insert(id);
            }

            fn build(&mut self, ctx: ElementTreeContext, element: &mut Element) -> Vec<Widget> {
                let children = element.build(&mut ElementBuildContext {
                    scheduler: &mut ctx.scheduler.with_strategy(self.scheduler),

                    element_tree: ctx.tree,
                    inheritance: ctx.inheritance,

                    callback_queue: self.callback_queue,

                    element_id: ctx.element_id,
                });

                if !self.is_first_root {
                    self.forgotten_elements.remove(ctx.element_id);
                }

                children
            }
        }

        self.tree.spawn_and_inflate(
            &mut InflateRootStrategy {
                is_first_root,

                scheduler: &mut self.scheduler,

                callback_queue: &self.callback_queue,
                needs_build: &mut self.needs_build,

                forgotten_elements: &mut FxHashSet::default(),
            },
            None,
            root,
        )
    }

    /// Update the UI tree.
    #[tracing::instrument(level = "trace", skip(self), fields(iteration = field::Empty))]
    pub fn update(&mut self) -> usize {
        let span = tracing::Span::current();

        let mut num_iteration = 0;

        // Rebuild the tree in a loop until it's fully settled. This is necessary as some
        // widgets being build may cause other widgets to be marked as dirty, which would
        // otherwise be missed in a single pass.
        while !self.rebuild_queue.is_empty() || self.flush_needs_build() {
            num_iteration += 1;

            if tracing::span_enabled!(tracing::Level::TRACE) {
                span.record("iteration", num_iteration);
            }

            self.flush_rebuilds();
        }

        self.flush_removals();

        num_iteration
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_callbacks(&mut self) {
        while let Ok(invoke) = self.callback_rx.try_recv() {
            let element_id = invoke.callback_id.element_id();

            let existed = self
                .tree
                .with(invoke.callback_id.element_id(), |ctx, element| {
                    let changed = element.call(
                        &mut ElementCallbackContext {
                            scheduler: &mut ctx.scheduler.with_strategy(&mut self.scheduler),

                            element_tree: ctx.tree,
                            inheritance: ctx.inheritance,

                            element_id: &element_id,
                        },
                        invoke.callback_id,
                        invoke.arg,
                    );

                    if changed {
                        tracing::trace!("element updated, queueing for rebuild");

                        // How often does the same element get callbacks multiple times? Is it
                        // worth checking if the last element is the same as the one we're about
                        // to queue?
                        self.rebuild_queue.push_back(element_id);
                    }
                })
                .is_some();

            if !existed {
                tracing::warn!("callback invoked on an element that does not exist");
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_needs_build(&mut self) -> bool {
        self.needs_build.process(|element_id| {
            if self.tree.contains(element_id) {
                tracing::trace!(?element_id, "queueing element for rebuild");

                self.rebuild_queue.push_back(element_id);
            } else {
                tracing::warn!("queued an element for rebuild, but it does not exist in the tree");
            }
        })
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_rebuilds(&mut self) {
        struct RebuildStrategy<'ctx, Sched> {
            scheduler: &'ctx mut Sched,

            callback_queue: &'ctx CallbackQueue,
            needs_build: &'ctx mut Dirty<ElementId>,

            rebuilt_elements: &'ctx mut FxHashSet<ElementId>,
            forgotten_elements: &'ctx mut FxHashSet<ElementId>,
        }

        impl<Sched> InflateStrategy for RebuildStrategy<'_, Sched>
        where
            Sched: ElementSchedulerStrategy,
        {
            fn on_spawned(&mut self, parent_id: Option<ElementId>, id: ElementId) {}

            fn on_updated(&mut self, id: ElementId) {}

            fn on_forgotten(&mut self, id: ElementId) {
                self.forgotten_elements.insert(id);
            }

            fn build(&mut self, ctx: ElementTreeContext, element: &mut Element) -> Vec<Widget> {
                let children = element.build(&mut ElementBuildContext {
                    scheduler: &mut ctx.scheduler.with_strategy(self.scheduler),

                    element_tree: ctx.tree,
                    inheritance: ctx.inheritance,

                    callback_queue: self.callback_queue,

                    element_id: ctx.element_id,
                });

                self.rebuilt_elements.insert(*ctx.element_id);
                self.forgotten_elements.remove(ctx.element_id);

                children
            }
        }

        // Keep track of which elements ended up being rebuilt, since build_and_realize
        // may end up rebuilding one that's currently in the queue.
        let mut rebuilt_elements = FxHashSet::default();

        rebuilt_elements.reserve(self.rebuild_queue.len().min(8));

        while let Some(element_id) = self.rebuild_queue.pop_front() {
            if rebuilt_elements.contains(&element_id) {
                tracing::trace!(
                    ?element_id,
                    "skipping element that was already rebuilt by another element"
                );

                continue;
            }

            if let Err(err) = self.tree.build_and_realize(
                &mut RebuildStrategy {
                    scheduler: &mut self.scheduler,

                    callback_queue: &self.callback_queue,
                    needs_build: &mut self.needs_build,

                    rebuilt_elements: &mut rebuilt_elements,
                    forgotten_elements: &mut self.forgotten_elements,
                },
                element_id,
            ) {
                match err {
                    InflateError::Broken | InflateError::Spawn(SpawnElementError::Broken) => {
                        tracing::error!("the tree is in an invalid state, aborting update");
                        return;
                    }

                    InflateError::Missing(element_id) => {
                        tracing::warn!(?element_id, "element was missing from the tree");
                    }

                    InflateError::InUse(element_id) => {
                        panic!(
                            "failed to rebuild element as it was in use: {:?}",
                            element_id
                        );
                    }
                }
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn flush_removals(&mut self) {
        for element_id in self.forgotten_elements.drain() {
            if !self.tree.contains(element_id) {
                continue;
            }

            if let Err(errs) = self.tree.remove(element_id) {
                for err in errs {
                    tracing::error!(?err, "an error occured while removing an element");
                }
            }
        }
    }
}

use std::{collections::VecDeque, hash::BuildHasherDefault, sync::Arc};

use agui_core::{
    callback::strategies::CallbackStrategy,
    element::{Element, ElementBuildContext, ElementId, ElementMountContext},
    engine::elements::{
        context::{ElementTreeContext, ElementTreeMountContext},
        scheduler::ElementSchedulerStrategy,
        strategies::InflateElementStrategy,
    },
    widget::Widget,
};
use rustc_hash::{FxHashSet, FxHasher};
use slotmap::SparseSecondaryMap;

pub struct RebuildStrategy<'rebuild, Sched> {
    pub scheduler: &'rebuild mut Sched,
    pub callbacks: &'rebuild Arc<dyn CallbackStrategy>,

    pub spawned_elements: &'rebuild mut VecDeque<ElementId>,
    pub updated_elements:
        &'rebuild mut SparseSecondaryMap<ElementId, (), BuildHasherDefault<FxHasher>>,

    pub rebuilt_elements: &'rebuild mut FxHashSet<ElementId>,
}

impl<Sched> InflateElementStrategy for RebuildStrategy<'_, Sched>
where
    Sched: ElementSchedulerStrategy,
{
    type Definition = Widget;

    #[tracing::instrument(level = "debug", skip(self, ctx))]
    fn mount(&mut self, ctx: ElementTreeMountContext, definition: Self::Definition) -> Element {
        self.spawned_elements.push_back(*ctx.element_id);

        let mut element = definition.create_element();

        element.mount(&mut ElementMountContext {
            element_tree: ctx.tree,

            parent_element_id: ctx.parent_element_id,
            element_id: ctx.element_id,
        });

        element
    }

    #[tracing::instrument(level = "debug", skip(self))]
    fn try_update(
        &mut self,
        id: ElementId,
        element: &mut Element,
        definition: &Self::Definition,
    ) -> agui_core::element::ElementComparison {
        self.updated_elements.insert(id, ());

        element.update(definition)
    }

    #[tracing::instrument(level = "debug", skip(self, ctx))]
    fn build(&mut self, ctx: ElementTreeContext, element: &mut Element) -> Vec<Widget> {
        self.rebuilt_elements.insert(*ctx.element_id);

        element.build(&mut ElementBuildContext {
            scheduler: &mut ctx.scheduler.with_strategy(self.scheduler),
            callbacks: self.callbacks,

            element_tree: ctx.tree,
            inheritance: ctx.inheritance,

            element_id: ctx.element_id,
        })
    }
}

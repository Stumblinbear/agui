use std::{collections::VecDeque, sync::Arc};

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

pub struct InflateRoot<'inflate, Sched> {
    pub scheduler: &'inflate mut Sched,
    pub callbacks: &'inflate Arc<dyn CallbackStrategy>,

    pub spawned_elements: &'inflate mut VecDeque<ElementId>,
}

impl<Sched> InflateElementStrategy for InflateRoot<'_, Sched>
where
    Sched: ElementSchedulerStrategy,
{
    type Definition = Widget;

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

    fn try_update(
        &mut self,
        _: ElementId,
        _: &mut Element,
        _: &Self::Definition,
    ) -> agui_core::element::ElementComparison {
        unreachable!("elements should never be updated while inflating the first root widget");
    }

    fn build(&mut self, ctx: ElementTreeContext, element: &mut Element) -> Vec<Widget> {
        let children = element.build(&mut ElementBuildContext {
            scheduler: &mut ctx.scheduler.with_strategy(self.scheduler),
            callbacks: self.callbacks,

            element_tree: ctx.tree,
            inheritance: ctx.inheritance,

            element_id: ctx.element_id,
        });

        children
    }
}

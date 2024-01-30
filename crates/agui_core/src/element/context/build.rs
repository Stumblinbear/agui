use std::{any::TypeId, future::Future};

use crate::{
    callback::CallbackQueue,
    element::{
        inherited::ElementInherited, Element, ElementBuilder, ElementId, ElementTaskContext,
    },
    engine::elements::scheduler::ElementScheduler,
    inheritance::InheritanceManager,
    task::{context::ContextSpawnElementTask, error::TaskError, TaskHandle},
    util::tree::Tree,
    widget::AnyWidget,
};

use super::{ContextElement, ContextElements};

pub struct ElementBuildContext<'ctx> {
    pub scheduler: &'ctx mut ElementScheduler<'ctx>,

    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub inheritance: &'ctx mut InheritanceManager,
    pub callback_queue: &'ctx CallbackQueue,

    pub element_id: &'ctx ElementId,
}

impl ContextElements for ElementBuildContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for ElementBuildContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextSpawnElementTask for ElementBuildContext<'_> {
    fn spawn_task<Fut>(
        &mut self,
        func: impl FnOnce(ElementTaskContext) -> Fut + 'static,
    ) -> Result<TaskHandle<()>, TaskError>
    where
        Fut: Future<Output = ()> + 'static,
    {
        self.scheduler.spawn_task(func)
    }
}

impl ElementBuildContext<'_> {
    pub fn find_inherited_widget<I>(
        &self,
    ) -> Option<&<<I as ElementBuilder>::Element as ElementInherited>::Data>
    where
        I: AnyWidget + ElementBuilder,
        <I as ElementBuilder>::Element: ElementInherited,
    {
        if let Some(element_id) = self.inheritance.find_type(
            *self.element_id,
            &TypeId::of::<<<I as ElementBuilder>::Element as ElementInherited>::Data>(),
        ) {
            let inherited_element = self
                .elements()
                .get(element_id)
                .expect("found an inherited element but it does not exist exist in the tree");

            debug_assert!(
                matches!(inherited_element, Element::Inherited(_)),
                "widget did not create an inherited element"
            );

            let Some(element) = inherited_element.downcast::<<I as ElementBuilder>::Element>()
            else {
                panic!("inherited element downcast failed");
            };

            Some(element.inherited_data())
        } else {
            None
        }
    }

    pub fn depend_on_inherited_widget<I>(
        &mut self,
    ) -> Option<&<<I as ElementBuilder>::Element as ElementInherited>::Data>
    where
        I: AnyWidget + ElementBuilder,
        <I as ElementBuilder>::Element: ElementInherited,
    {
        if let Some(element_id) = self.inheritance.depend_on_type(
            *self.element_id,
            TypeId::of::<<<I as ElementBuilder>::Element as ElementInherited>::Data>(),
        ) {
            let inherited_element = self
                .elements()
                .get(element_id)
                .expect("found an inherited element but it does not exist exist in the tree");

            debug_assert!(
                matches!(inherited_element, Element::Inherited(_)),
                "widget did not create an inherited element"
            );

            let Some(element) = inherited_element.downcast::<<I as ElementBuilder>::Element>()
            else {
                panic!("inherited element downcast failed");
            };

            Some(element.inherited_data())
        } else {
            None
        }
    }
}

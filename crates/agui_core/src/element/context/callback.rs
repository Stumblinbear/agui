use std::{any::TypeId, future::Future};

use crate::{
    element::{
        inherited::ElementInherited, ContextDirtyElement, ContextElement, Element, ElementBuilder,
        ElementId, ElementTaskContext, ElementType,
    },
    engine::{widgets::bindings::ElementSchedulerBinding, Dirty},
    inheritance::InheritanceManager,
    task::{context::ContextSpawnElementTask, error::TaskError, TaskHandle},
    util::tree::Tree,
    widget::AnyWidget,
};

use super::ContextElements;

pub struct ElementCallbackContext<'ctx> {
    pub(crate) scheduler: &'ctx mut dyn ElementSchedulerBinding,

    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub inheritance: &'ctx InheritanceManager,
    pub needs_build: &'ctx mut Dirty<ElementId>,

    pub element_id: &'ctx ElementId,
}

impl ContextElements for ElementCallbackContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for ElementCallbackContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextDirtyElement for ElementCallbackContext<'_> {
    fn mark_needs_build(&mut self) {
        tracing::trace!(element_id = ?self.element_id, "element needs build");

        self.needs_build.insert(*self.element_id);
    }
}

impl ContextSpawnElementTask for ElementCallbackContext<'_> {
    fn spawn_task<Fut>(
        &self,
        func: impl FnOnce(ElementTaskContext) -> Fut + 'static,
    ) -> Result<TaskHandle<()>, TaskError>
    where
        Fut: Future<Output = ()> + 'static,
    {
        self.scheduler.spawn_task(
            *self.element_id,
            Box::pin(func(ElementTaskContext {
                element_id: *self.element_id,
                needs_build: self.needs_build.clone(),
            })),
        )
    }
}

impl ElementCallbackContext<'_> {
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
                matches!(inherited_element.as_ref(), ElementType::Inherited(_)),
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

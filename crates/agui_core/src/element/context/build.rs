use std::{any::TypeId, future::Future, rc::Rc};

use futures::{future::RemoteHandle, task::SpawnError};

use crate::{
    callback::CallbackQueue,
    element::{Element, ElementId, ElementType},
    engine::{bindings::SchedulerBinding, Dirty},
    inheritance::InheritanceManager,
    util::tree::Tree,
    widget::AnyWidget,
};

use super::{ContextElement, ContextElements};

pub struct ElementBuildContext<'ctx> {
    pub(crate) scheduler: &'ctx mut dyn SchedulerBinding,

    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub inheritance: &'ctx mut InheritanceManager,
    pub callback_queue: &'ctx CallbackQueue,

    pub needs_build: &'ctx mut Dirty<ElementId>,

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

impl ElementBuildContext<'_> {
    pub fn spawn_local<Fut>(&mut self, future: Fut) -> Result<RemoteHandle<()>, SpawnError>
    where
        Fut: Future<Output = ()> + 'static,
    {
        self.scheduler
            .spawn_local_task(*self.element_id, Box::pin(future))
    }

    pub fn spawn_shared<Fut>(&mut self, future: Fut) -> Result<RemoteHandle<()>, SpawnError>
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.scheduler
            .spawn_shared_task(*self.element_id, Box::pin(future))
    }

    pub fn find_inherited_widget<I>(&self) -> Option<Rc<I>>
    where
        I: AnyWidget,
    {
        if let Some(element_id) = self
            .inheritance
            .find_type(*self.element_id, &TypeId::of::<I>())
        {
            let inherited_element = self
                .elements()
                .get(element_id)
                .expect("found an inherited widget but it does not exist exist in the tree");

            if !matches!(inherited_element.as_ref(), ElementType::Inherited(_)) {
                panic!("widget did not create an inherited element");
            }

            let Some(widget) = inherited_element.widget().downcast::<I>() else {
                panic!("inherited widget downcast failed");
            };

            Some(widget)
        } else {
            None
        }
    }

    pub fn depend_on_inherited_widget<I>(&mut self) -> Option<Rc<I>>
    where
        I: AnyWidget,
    {
        if let Some(element_id) = self
            .inheritance
            .depend_on_type(*self.element_id, TypeId::of::<I>())
        {
            let inherited_element = self
                .elements()
                .get(element_id)
                .expect("found an inherited widget but it does not exist exist in the tree");

            if !matches!(inherited_element.as_ref(), ElementType::Inherited(_)) {
                panic!("widget did not create an inherited element");
            }

            let Some(widget) = inherited_element.widget().downcast::<I>() else {
                panic!("inherited widget downcast failed");
            };

            Some(widget)
        } else {
            None
        }
    }
}

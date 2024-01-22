use futures::future::Future;

use crate::{
    element::{ContextDirtyElement, ContextElement, Element, ElementId, ElementTaskContext},
    engine::{bindings::ElementSchedulerBinding, Dirty},
    task::{context::ContextSpawnElementTask, error::TaskError, TaskHandle},
    util::tree::Tree,
};

use super::ContextElements;

pub struct ElementCallbackContext<'ctx> {
    pub(crate) scheduler: &'ctx mut dyn ElementSchedulerBinding,

    pub element_tree: &'ctx Tree<ElementId, Element>,
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

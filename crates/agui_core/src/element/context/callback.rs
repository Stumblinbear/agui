use futures::{future::Future, future::RemoteHandle, task::SpawnError};

use crate::{
    element::{ContextDirtyElement, ContextElement, Element, ElementId},
    engine::{bindings::LocalSchedulerBinding, Dirty},
    util::tree::Tree,
};

use super::ContextElements;

pub struct ElementCallbackContext<'ctx> {
    pub(crate) scheduler: &'ctx mut dyn LocalSchedulerBinding,

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

impl ElementCallbackContext<'_> {
    pub fn spawn_task<Fut>(&self, future: Fut) -> Result<RemoteHandle<()>, SpawnError>
    where
        Fut: Future<Output = ()> + 'static,
    {
        self.scheduler
            .spawn_task(*self.element_id, Box::pin(future))
    }
}

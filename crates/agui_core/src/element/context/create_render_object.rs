use std::future::Future;

use futures::{future::RemoteHandle, task::SpawnError};

use crate::{element::ElementId, engine::bindings::SharedSchedulerBinding};

use super::ContextElement;

pub struct RenderObjectCreateContext<'ctx> {
    pub(crate) scheduler: &'ctx mut dyn SharedSchedulerBinding,

    pub element_id: &'ctx ElementId,
}

impl ContextElement for RenderObjectCreateContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl RenderObjectCreateContext<'_> {
    pub fn spawn_task<Fut>(&self, future: Fut) -> Result<RemoteHandle<()>, SpawnError>
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.scheduler
            .spawn_task(*self.element_id, Box::pin(future))
    }
}

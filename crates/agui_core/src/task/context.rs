use std::future::Future;

use crate::{
    element::{ElementTaskContext, RenderObjectTaskContext},
    task::{error::TaskError, TaskHandle},
};

pub trait ContextSpawnElementTask {
    fn spawn_task<Fut>(
        &self,
        func: impl FnOnce(ElementTaskContext) -> Fut + 'static,
    ) -> Result<TaskHandle<()>, TaskError>
    where
        Fut: Future<Output = ()> + 'static;
}

pub trait ContextSpawnRenderingTask {
    fn spawn_task<Fut>(
        &self,
        func: impl FnOnce(RenderObjectTaskContext) -> Fut + 'static,
    ) -> Result<TaskHandle<()>, TaskError>
    where
        Fut: Future<Output = ()> + Send + 'static;
}

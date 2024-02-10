use std::future::Future;

use crate::{
    element::{ElementTaskContext, RenderingTaskContext},
    task::{error::TaskError, TaskHandle},
};

pub trait ContextSpawnElementTask {
    fn spawn_task<Fut>(
        &mut self,
        func: impl FnOnce(ElementTaskContext) -> Fut + 'static,
    ) -> Result<TaskHandle<()>, TaskError>
    where
        Fut: Future<Output = ()> + 'static;
}

pub trait ContextSpawnRenderingTask {
    fn spawn_task<Fut>(
        &mut self,
        func: impl FnOnce(RenderingTaskContext) -> Fut + Send + 'static,
    ) -> Result<TaskHandle<()>, TaskError>
    where
        Fut: Future<Output = ()> + Send + 'static;
}

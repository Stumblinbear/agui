use std::{future::Future, pin::Pin};

use crate::{
    render::RenderObjectId,
    task::{error::TaskError, TaskHandle},
};

pub type RenderingTask = Pin<Box<dyn Future<Output = ()> + Send>>;

#[allow(unused_variables)]
pub trait RenderingSchedulerBinding {
    fn spawn_task(
        &self,
        id: RenderObjectId,
        task: RenderingTask,
    ) -> Result<TaskHandle<()>, TaskError>;
}

impl RenderingSchedulerBinding for () {
    fn spawn_task(&self, _: RenderObjectId, _: RenderingTask) -> Result<TaskHandle<()>, TaskError> {
        Err(TaskError::no_scheduler())
    }
}

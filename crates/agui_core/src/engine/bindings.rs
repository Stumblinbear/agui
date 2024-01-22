use futures::{future::BoxFuture, prelude::future::LocalBoxFuture};

use crate::{
    element::ElementId,
    task::{error::TaskError, TaskHandle},
};

#[allow(unused_variables)]
pub trait ElementBinding {
    fn on_element_spawned(&mut self, parent_id: Option<ElementId>, id: ElementId) {}

    fn on_element_build(&mut self, id: ElementId) {}

    fn on_element_needs_rebuild(&mut self, id: ElementId) {}

    fn on_element_destroyed(&mut self, id: ElementId) {}
}

impl ElementBinding for () {}

pub type ElementTask = LocalBoxFuture<'static, ()>;

pub type RenderingTask = BoxFuture<'static, ()>;

#[allow(unused_variables)]
pub trait ElementSchedulerBinding {
    fn spawn_task(&self, id: ElementId, task: ElementTask) -> Result<TaskHandle<()>, TaskError>;
}

impl ElementSchedulerBinding for () {
    fn spawn_task(&self, _: ElementId, _: ElementTask) -> Result<TaskHandle<()>, TaskError> {
        Err(TaskError::no_scheduler())
    }
}

#[allow(unused_variables)]
pub trait RenderingSchedulerBinding {
    fn spawn_task(&self, id: ElementId, task: RenderingTask) -> Result<TaskHandle<()>, TaskError>;
}

impl RenderingSchedulerBinding for () {
    fn spawn_task(&self, _: ElementId, _: RenderingTask) -> Result<TaskHandle<()>, TaskError> {
        Err(TaskError::no_scheduler())
    }
}

use std::{future::Future, pin::Pin};

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

pub type ElementTask = Pin<Box<dyn Future<Output = ()>>>;

#[allow(unused_variables)]
pub trait ElementSchedulerBinding {
    fn spawn_task(&self, id: ElementId, task: ElementTask) -> Result<TaskHandle<()>, TaskError>;
}

impl ElementSchedulerBinding for () {
    fn spawn_task(&self, _: ElementId, _: ElementTask) -> Result<TaskHandle<()>, TaskError> {
        Err(TaskError::no_scheduler())
    }
}

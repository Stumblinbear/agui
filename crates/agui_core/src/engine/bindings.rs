use std::{future::Future, pin::Pin};

use futures::{future::RemoteHandle, task::SpawnError};

use crate::element::ElementId;

#[allow(unused_variables)]
pub trait ElementBinding {
    fn on_element_spawned(&mut self, parent_id: Option<ElementId>, id: ElementId) {}

    fn on_element_build(&mut self, id: ElementId) {}

    fn on_element_needs_rebuild(&mut self, id: ElementId) {}

    fn on_element_destroyed(&mut self, id: ElementId) {}
}

impl ElementBinding for () {}

#[allow(unused_variables)]
pub trait SchedulerBinding {
    fn spawn_local_task(
        &mut self,
        id: ElementId,
        future: Pin<Box<dyn Future<Output = ()> + 'static>>,
    ) -> Result<RemoteHandle<()>, SpawnError>;

    fn spawn_shared_task(
        &mut self,
        id: ElementId,
        future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    ) -> Result<RemoteHandle<()>, SpawnError>;
}

impl SchedulerBinding for () {
    fn spawn_local_task(
        &mut self,
        _: ElementId,
        _: Pin<Box<dyn Future<Output = ()> + 'static>>,
    ) -> Result<RemoteHandle<()>, SpawnError> {
        Err(SpawnError::shutdown())
    }

    fn spawn_shared_task(
        &mut self,
        _: ElementId,
        _: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    ) -> Result<RemoteHandle<()>, SpawnError> {
        Err(SpawnError::shutdown())
    }
}

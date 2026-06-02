use std::{any::Any, future::Future, pin::Pin, sync::mpsc};

use crate::{routing_id::RoutingPath, task::TaskHandle};

/// A message addressed to an element, queued on the event channel and routed to that element on the
/// next drain.
pub type TaskEventMessage = (RoutingPath, Box<dyn Any>);

/// Sender half of the event channel, cloned into a [`TaskCtx`] so a running task can post a message
/// back to its element.
pub type EventSender = mpsc::Sender<TaskEventMessage>;

pub type TaskFuture = Pin<Box<dyn Future<Output = ()>>>;

pub trait TaskScheduler {
    fn event_tx(&self) -> EventSender;

    /// Spawn an async task to perform the given work on the current thread. These tasks will be driven
    /// on the same thread that it was spawned on.
    fn spawn(&mut self, func: TaskFuture) -> Result<TaskHandle, Box<dyn std::error::Error>>;

    /// An owned scheduler handle that outlives the current build frame.
    fn deferred(&self) -> Box<dyn TaskScheduler>;
}

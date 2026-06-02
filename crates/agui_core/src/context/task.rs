use std::any::Any;

use crate::{routing_id::RoutingPath, task::scheduler::EventSender};

pub struct TaskCtx {
    event_tx: EventSender,
    routing_path: RoutingPath,
}

impl TaskCtx {
    pub fn new(event_tx: EventSender, routing_path: RoutingPath) -> Self {
        Self {
            event_tx,
            routing_path,
        }
    }

    /// Post a message back to the element that spawned this task. It is delivered on the next event
    /// drain and routed to that element.
    pub fn send<M>(&self, message: M)
    where
        M: Any,
    {
        // A closed channel means the tree is gone; dropping the message is the right thing.
        let _ = self
            .event_tx
            .send((self.routing_path.clone(), Box::new(message)));
    }
}

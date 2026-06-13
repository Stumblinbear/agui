use std::any::Any;

use crate::{element::RoutingTarget, scheduling::EventSender};

pub struct TaskCtx {
    event_tx: EventSender,
    routing_target: RoutingTarget,
}

impl TaskCtx {
    pub fn new(event_tx: EventSender, routing_target: RoutingTarget) -> Self {
        Self {
            event_tx,
            routing_target,
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
            .send((self.routing_target.clone(), Box::new(message)));
    }
}

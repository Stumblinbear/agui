use std::any::Any;

use agui_core::tree::NodeHandle;

use crate::scheduling::EventSender;

/// The context a spawned task receives, for posting a message back to the element that spawned it.
pub struct TaskCtx {
    event_tx: EventSender,
    target: NodeHandle,
}

impl TaskCtx {
    pub(crate) fn new(event_tx: EventSender, target: NodeHandle) -> Self {
        Self { event_tx, target }
    }

    /// Posts `message` back to the element that spawned this task, delivered on the next event drain. A
    /// message to an element that has since been removed is dropped.
    pub fn send<M: Any>(&self, message: M) {
        // A closed channel means the tree is gone, so dropping the message is correct.
        let _ = self.event_tx.send((self.target, Box::new(message)));
    }
}

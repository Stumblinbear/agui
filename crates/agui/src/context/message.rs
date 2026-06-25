use std::any::Any;

use agui_core::tree::NodeHandle;

use crate::pipeline::build_tree::BuildQueue;

/// The context passed to an element while it handles a message: the payload and the build queue. It has no
/// cursor, so a message cannot restructure the tree.
pub struct MessageCtx<'a> {
    message: Option<Box<dyn Any>>,
    handle: NodeHandle,
    queue: &'a mut BuildQueue,
}

impl<'a> MessageCtx<'a> {
    /// Wraps `message` for delivery to the element at `handle`, with the build queue the element marks itself
    /// in if it requests a rebuild. The driver builds one to dispatch a message by handle.
    pub fn new(message: Box<dyn Any>, handle: NodeHandle, queue: &'a mut BuildQueue) -> Self {
        Self {
            message: Some(message),
            handle,
            queue,
        }
    }

    /// Takes the message payload as a `T`.
    ///
    /// # Panics
    /// If the payload was already taken, or is not a `T`.
    pub fn consume<T: Any>(&mut self) -> T {
        let message = self
            .message
            .take()
            .expect("message has already been consumed");
        *message.downcast::<T>().expect("message was not a T")
    }

    /// Queues this element to rebuild on the next flush.
    pub fn request_rebuild(&mut self) {
        self.queue.mark_rebuild(self.handle);
    }
}

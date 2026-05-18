use std::any::Any;

pub struct MessageCtx {
    message: Option<Box<dyn Any>>,
    rebuild_requested: bool,
}

impl MessageCtx {
    pub fn new(message: Box<dyn Any>) -> Self {
        Self {
            message: Some(message),
            rebuild_requested: false,
        }
    }

    /// Take the message payload, downcasting to the expected type. Panics if already
    /// consumed or if the type doesn't match.
    pub fn consume<T>(&mut self) -> T
    where
        T: Any,
    {
        let msg = self
            .message
            .take()
            .expect("message has already been consumed");

        *msg.downcast::<T>().expect("message downcast failed")
    }

    /// Mark the dispatched element as needing a rebuild. The dispatcher reads this
    /// after `dispatch` returns and queues the dispatched path for reconciliation.
    pub fn request_rebuild(&mut self) {
        self.rebuild_requested = true;
    }

    /// Whether `request_rebuild` was called during the dispatch.
    pub fn rebuild_requested(&self) -> bool {
        self.rebuild_requested
    }
}

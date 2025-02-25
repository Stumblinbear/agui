use std::any::Any;

use crate::routing_id::RoutingId;

pub struct MessageCtx<'a> {
    path: &'a [RoutingId],

    message: Box<dyn Any>,
}

impl<'a> MessageCtx<'a> {
    pub fn new(path: &'a [RoutingId], message: Box<dyn Any>) -> Self {
        Self { path, message }
    }

    pub fn routing_id(&self) -> Option<u16> {
        self.path.first().copied().map(RoutingId::get)
    }

    pub fn consume<T>(self) -> T
    where
        T: Any,
    {
        assert!(
            self.path.is_empty(),
            "cannot take message as it is destined for a child",
        );

        *self
            .message
            .downcast::<T>()
            .expect("message downcast failed")
    }
}

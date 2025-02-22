use std::any::Any;

use crate::view_id::ViewId;

pub struct MessageCtx<'a> {
    path: &'a [ViewId],

    message: Box<dyn Any>,
}

impl<'a> MessageCtx<'a> {
    pub fn new(path: &'a [ViewId], message: Box<dyn Any>) -> Self {
        Self { path, message }
    }

    pub fn routing_id(&self) -> Option<u16> {
        self.path.first().copied().map(ViewId::get)
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

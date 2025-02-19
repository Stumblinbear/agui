use std::any::Any;

use crate::{element::Element, view_id::ViewId};

pub struct MessageCtx<'a> {
    pub element: &'a mut Element,

    path: &'a [ViewId],

    message: Option<Box<dyn Any>>,
}

impl<'a> MessageCtx<'a> {
    pub fn new(element: &'a mut Element, path: &'a [ViewId], message: Box<dyn Any>) -> Self {
        Self {
            element,

            path,

            message: Some(message),
        }
    }

    pub fn routing_id(&self) -> Option<u32> {
        self.path.first().copied().map(ViewId::get)
    }

    pub fn take<T>(mut self) -> T
    where
        T: Any,
    {
        assert!(
            self.path.is_empty(),
            "cannot take message as it is destined for a child",
        );

        *self
            .message
            .take()
            .expect("message already taken")
            .downcast::<T>()
            .expect("message downcast failed")
    }

    pub fn child(mut self, view_id: ViewId, func: impl FnOnce(MessageCtx)) {
        let child = self
            .element
            .children
            .get_mut(view_id.get() as usize)
            .expect("child not found");

        func(MessageCtx {
            element: child,

            path: self.path,

            message: self.message,
        });
    }
}

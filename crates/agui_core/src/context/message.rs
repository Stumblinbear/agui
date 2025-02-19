use std::{
    any::Any,
    ops::{Deref, DerefMut},
};

use crate::{tree::Tree, view_id::ViewId};

pub struct MessageCtx<'a> {
    tree: &'a mut Tree,
    path: &'a [ViewId],

    message: Option<Box<dyn Any>>,
}

impl<'a> MessageCtx<'a> {
    pub fn new(tree: &'a mut Tree, path: &'a [ViewId], message: Box<dyn Any>) -> Self {
        Self {
            tree,
            path,

            message: Some(message),
        }
    }

    pub fn routing_id(&self) -> Option<ViewId> {
        self.path.first().copied()
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
            .tree
            .children
            .get_mut(view_id.get())
            .expect("child not found");

        func(MessageCtx {
            tree: child,
            path: &mut self.path,

            message: self.message,
        });
    }
}

impl<'a> Deref for MessageCtx<'a> {
    type Target = Tree;

    fn deref(&self) -> &Self::Target {
        self.tree
    }
}

impl<'a> DerefMut for MessageCtx<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.tree
    }
}

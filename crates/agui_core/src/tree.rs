use std::{
    any::Any,
    ops::{Deref, DerefMut},
};

use crate::{size::Size, view::ViewLifecycle, view_id::ViewId};

pub struct TreeState(Option<Box<dyn Any>>);

impl TreeState {
    pub fn none() -> Self {
        Self(None)
    }

    pub fn new<T>(value: T) -> Self
    where
        T: Any,
    {
        Self(Some(Box::new(value)))
    }
}

impl Deref for TreeState {
    type Target = Option<Box<dyn Any>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TreeState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct Tree {
    state: TreeState,

    pub children: Vec<Tree>,

    pub size: Size,
}

impl Tree {
    pub fn empty() -> Self {
        Self {
            state: TreeState::none(),

            children: Vec::new(),

            size: Size::ZERO,
        }
    }

    pub fn new(view: &impl ViewLifecycle) -> Self {
        Self {
            state: view.state(),

            children: view.children(),

            size: Size::ZERO,
        }
    }

    pub fn update(&mut self, view: &impl ViewLifecycle) {
        // TODO(trevin): diff the tree

        self.state = view.state();
        self.children = view.children();
    }

    pub fn state<T>(&self) -> &T
    where
        T: Any,
    {
        self.state
            .as_ref()
            .expect("no state")
            .downcast_ref()
            .expect("node state downcast failed")
    }

    pub fn state_mut<T>(&mut self) -> &mut T
    where
        T: Any,
    {
        self.state
            .as_mut()
            .expect("no state")
            .downcast_mut()
            .expect("node state downcast failed")
    }

    pub fn child(&self, view_id: ViewId) -> &Tree {
        &self.children[view_id.get()]
    }

    pub fn child_mut(&mut self, view_id: ViewId) -> &mut Tree {
        &mut self.children[view_id.get()]
    }
}

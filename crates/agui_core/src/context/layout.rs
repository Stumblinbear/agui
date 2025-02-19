use std::ops::{Deref, DerefMut};

use crate::{size::Size, tree::Tree, view_id::ViewId};

pub struct LayoutCtx<'a> {
    tree: &'a mut Tree,
}

impl<'a> LayoutCtx<'a> {
    pub fn new(tree: &'a mut Tree) -> Self {
        Self { tree }
    }

    pub fn child(&mut self, view_id: ViewId, func: impl FnOnce(LayoutCtx)) -> ChildLayoutRef {
        let child = self
            .tree
            .children
            .get_mut(view_id.get())
            .expect("child not found");

        func(LayoutCtx { tree: child });

        ChildLayoutRef { tree: child }
    }
}

impl Deref for LayoutCtx<'_> {
    type Target = Tree;

    fn deref(&self) -> &Self::Target {
        self.tree
    }
}

impl DerefMut for LayoutCtx<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.tree
    }
}

pub struct ChildLayoutRef<'a> {
    tree: &'a Tree,
}

impl ChildLayoutRef<'_> {
    pub fn size(self) -> Size {
        self.tree.size
    }
}

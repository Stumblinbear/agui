use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
};

use crate::{tree::Tree, view_id::ViewId};

pub struct UpdateCtx<'a> {
    tree: &'a mut Tree,
    path: &'a mut VecDeque<ViewId>,
}

impl<'a> UpdateCtx<'a> {
    pub fn new(tree: &'a mut Tree, path: &'a mut VecDeque<ViewId>) -> Self {
        Self { tree, path }
    }

    pub fn child(&mut self, view_id: ViewId, func: impl FnOnce(UpdateCtx)) {
        self.path.push_back(view_id);

        let child = self
            .tree
            .children
            .get_mut(view_id.get())
            .expect("child not found");

        func(UpdateCtx {
            tree: child,
            path: self.path,
        });

        self.path.pop_back();
    }
}

impl Deref for UpdateCtx<'_> {
    type Target = Tree;

    fn deref(&self) -> &Self::Target {
        self.tree
    }
}

impl DerefMut for UpdateCtx<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.tree
    }
}

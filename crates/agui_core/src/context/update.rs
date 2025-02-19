use std::collections::VecDeque;

use crate::view_id::ViewId;

pub struct UpdateCtx<'a> {
    path: &'a mut VecDeque<ViewId>,
}

impl<'a> UpdateCtx<'a> {
    pub fn new(path: &'a mut VecDeque<ViewId>) -> Self {
        Self { path }
    }

    pub fn path(&self) -> impl Iterator<Item = &ViewId> {
        self.path.iter()
    }

    pub(crate) fn with_view(&mut self, id: ViewId, func: impl FnOnce(UpdateCtx)) {
        self.path.push_back(id);

        func(UpdateCtx { path: self.path });

        self.path.pop_back();
    }
}

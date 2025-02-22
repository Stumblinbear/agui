use std::{collections::VecDeque, sync::mpsc};

use crate::view_id::ViewId;

pub struct UpdateCtx<'a> {
    event_tx: &'a mpsc::Sender<()>,

    path: &'a mut VecDeque<ViewId>,
}

impl<'a> UpdateCtx<'a> {
    pub fn new(event_tx: &'a mpsc::Sender<()>, path: &'a mut VecDeque<ViewId>) -> Self {
        Self { event_tx, path }
    }

    pub fn event_tx(&self) -> mpsc::Sender<()> {
        self.event_tx.clone()
    }

    pub fn path(&self) -> impl DoubleEndedIterator<Item = &ViewId> + ExactSizeIterator {
        self.path.iter()
    }

    pub(crate) fn with_view(&mut self, id: ViewId, func: impl FnOnce(&mut UpdateCtx)) {
        self.path.push_back(id);

        func(self);

        self.path.pop_back();
    }
}

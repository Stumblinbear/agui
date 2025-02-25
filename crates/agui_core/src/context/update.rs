use std::{collections::VecDeque, sync::mpsc};

use crate::routing_id::RoutingId;

pub struct UpdateCtx<'a> {
    event_tx: &'a mpsc::Sender<()>,

    path: &'a mut VecDeque<RoutingId>,
}

impl<'a> UpdateCtx<'a> {
    pub fn new(event_tx: &'a mpsc::Sender<()>, path: &'a mut VecDeque<RoutingId>) -> Self {
        Self { event_tx, path }
    }

    pub fn event_tx(&self) -> mpsc::Sender<()> {
        self.event_tx.clone()
    }

    pub fn path(&self) -> impl DoubleEndedIterator<Item = &RoutingId> + ExactSizeIterator {
        self.path.iter()
    }

    pub fn with_routing_id<T>(
        &mut self,
        id: RoutingId,
        func: impl FnOnce(&mut UpdateCtx) -> T,
    ) -> T {
        self.path.push_back(id);

        let ret = func(self);

        self.path.pop_back();

        ret
    }
}

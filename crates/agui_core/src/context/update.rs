use std::{any::Any, collections::VecDeque, rc::Rc, sync::mpsc};

use crate::{driver::Driver, provide::ProvideScope, routing_id::RoutingId};

pub struct UpdateCtx<'a> {
    driver: &'a Rc<dyn Driver>,

    event_tx: &'a mpsc::Sender<()>,

    routing_path: &'a mut VecDeque<RoutingId>,

    provide_scope: &'a ProvideScope,
}

impl<'a> UpdateCtx<'a> {
    pub fn new(
        driver: &'a Rc<dyn Driver>,
        event_tx: &'a mpsc::Sender<()>,
        routing_path: &'a mut VecDeque<RoutingId>,
        provide_scope: &'a ProvideScope,
    ) -> Self {
        Self {
            driver,

            event_tx,

            routing_path,

            provide_scope,
        }
    }

    pub fn driver(&self) -> &Rc<dyn Driver> {
        self.driver
    }

    pub fn event_tx(&self) -> mpsc::Sender<()> {
        self.event_tx.clone()
    }

    pub fn routing_path(&self) -> impl DoubleEndedIterator<Item = &RoutingId> + ExactSizeIterator {
        self.routing_path.iter()
    }

    pub fn provide_scope(&self) -> &ProvideScope {
        self.provide_scope
    }

    pub fn get_provided<T>(&self) -> Option<&T>
    where
        T: Any,
    {
        self.provide_scope.get()
    }

    pub fn with_routing_id<T>(
        &mut self,
        id: RoutingId,
        func: impl FnOnce(&mut UpdateCtx) -> T,
    ) -> T {
        self.routing_path.push_back(id);

        let ret = func(self);

        self.routing_path.pop_back();

        ret
    }

    pub fn with_provided<V, T>(&mut self, value: Rc<V>, func: impl FnOnce(&mut UpdateCtx) -> T) -> T
    where
        V: Any,
    {
        let provide_scope = self.provide_scope.provide(value);

        func(&mut UpdateCtx {
            driver: self.driver,

            event_tx: self.event_tx,

            routing_path: self.routing_path,

            provide_scope: &provide_scope,
        })
    }
}

use std::{collections::VecDeque, rc::Rc, sync::mpsc};

use crate::{
    context::UpdateCtx, driver::Driver, element::Element, provide::ProvideScope,
    routing_id::RoutingId, view::View,
};

pub struct NoopTestDriver;

impl Driver for NoopTestDriver {}

pub struct TestHarness {
    pub driver: Rc<dyn Driver>,
    pub event_tx: mpsc::Sender<()>,
    pub path: VecDeque<RoutingId>,
    pub provide_scope: ProvideScope,

    pub root: Element,
}

impl TestHarness {
    pub fn mount<V>(view: &V) -> Self
    where
        V: View,
    {
        let driver: Rc<dyn Driver> = Rc::new(NoopTestDriver);
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();
        let provide_scope = ProvideScope::new();

        let root = Element::new(
            view,
            &mut UpdateCtx::new(&driver, &tx, &mut path, &provide_scope),
        );

        Self {
            driver,
            event_tx: tx,
            path,
            provide_scope,

            root,
        }
    }

    pub fn update<V>(&mut self, old_view: &V, new_view: &V)
    where
        V: View,
    {
        self.root.as_mut(old_view).update(
            new_view,
            &mut UpdateCtx::new(
                &self.driver,
                &self.event_tx,
                &mut self.path,
                &self.provide_scope,
            ),
        );
    }
}

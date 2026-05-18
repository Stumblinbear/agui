use std::{rc::Rc, sync::mpsc};

use crate::{
    context::{Dispatch, MessageCtx, UpdateCtx},
    driver::Driver,
    element::Element,
    provide::ProvideScope,
    routing_id::RoutingId,
    view::View,
};

pub struct NoopTestDriver;

impl Driver for NoopTestDriver {}

pub struct TestHarness {
    pub driver: Rc<dyn Driver>,
    pub event_tx: mpsc::Sender<()>,
    pub path: Vec<RoutingId>,
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
        let mut path = Vec::new();
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

    /// Dispatch a message along `path` to a view in the tree. Returns the
    /// [`MessageCtx`] so the caller can inspect [`MessageCtx::rebuild_requested`].
    pub fn dispatch_message<V>(
        &mut self,
        view: &V,
        path: &[RoutingId],
        message: Box<dyn std::any::Any>,
    ) -> MessageCtx
    where
        V: View,
    {
        let mut msg_ctx = MessageCtx::new(message);
        view.dispatch(&mut self.root, path, Dispatch::Message(&mut msg_ctx));
        msg_ctx
    }

    /// Dispatch a rebuild along `path` to a view in the tree.
    pub fn dispatch_rebuild<V>(&mut self, view: &V, path: &[RoutingId])
    where
        V: View,
    {
        let mut update_ctx = UpdateCtx::new(
            &self.driver,
            &self.event_tx,
            &mut self.path,
            &self.provide_scope,
        );
        view.dispatch(&mut self.root, path, Dispatch::Rebuild(&mut update_ctx));
    }
}

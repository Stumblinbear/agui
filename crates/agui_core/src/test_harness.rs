use std::{rc::Rc, sync::mpsc};

use crate::{
    context::{Dispatch, MessageCtx, UpdateCtx},
    driver::Driver,
    element::ElementNode,
    provide::ProvideScope,
    routing_id::RoutingId,
    widget::Widget,
};

pub struct NoopTestDriver;

impl Driver for NoopTestDriver {}

pub struct TestHarness<E> {
    pub driver: Rc<dyn Driver>,
    pub event_tx: mpsc::Sender<()>,
    pub path: Vec<RoutingId>,
    pub provide_scope: ProvideScope,

    pub root: ElementNode<E>,
}

impl<E> TestHarness<E> {
    pub fn mount<V>(widget: &V) -> Self
    where
        V: Widget<Element = E>,
    {
        let driver: Rc<dyn Driver> = Rc::new(NoopTestDriver);
        let (tx, _) = mpsc::channel();
        let mut path = Vec::new();
        let provide_scope = ProvideScope::new();

        let root =
            widget.create_element(&mut UpdateCtx::new(&driver, &tx, &mut path, &provide_scope));

        Self {
            driver,
            event_tx: tx,
            path,
            provide_scope,

            root: ElementNode::new(root),
        }
    }

    pub fn update<V>(&mut self, old_widget: &V, new_widget: &V)
    where
        V: Widget<Element = E>,
    {
        new_widget.update(
            &mut self.root.element,
            old_widget,
            &mut UpdateCtx::new(
                &self.driver,
                &self.event_tx,
                &mut self.path,
                &self.provide_scope,
            ),
        );
    }

    /// Dispatch a message along `path` to a widget in the tree. Returns the
    /// [`MessageCtx`] so the caller can inspect [`MessageCtx::rebuild_requested`].
    pub fn dispatch_message<V>(
        &mut self,
        widget: &V,
        path: &[RoutingId],
        message: Box<dyn std::any::Any>,
    ) -> MessageCtx
    where
        V: Widget<Element = E>,
    {
        let mut msg_ctx = MessageCtx::new(message);
        widget.dispatch(
            &mut self.root.element,
            path,
            Dispatch::Message(&mut msg_ctx),
        );
        msg_ctx
    }

    /// Dispatch a rebuild along `path` to a widget in the tree.
    pub fn dispatch_rebuild<V>(&mut self, widget: &V, path: &[RoutingId])
    where
        V: Widget<Element = E>,
    {
        let mut update_ctx = UpdateCtx::new(
            &self.driver,
            &self.event_tx,
            &mut self.path,
            &self.provide_scope,
        );
        widget.dispatch(
            &mut self.root.element,
            path,
            Dispatch::Rebuild(&mut update_ctx),
        );
    }
}

use std::{any::Any, rc::Rc, sync::mpsc};

use crate::{
    context::{Dispatch, MessageCtx, UpdateCtx},
    element::Element,
    provide::ProvideScope,
    routing_id::RoutingPath,
    view::View,
};

pub trait Driver {}

pub fn dispatch_messages(
    root_element: &mut Element,
    root_view: &impl View,
    messages: impl Iterator<Item = (RoutingPath, Box<dyn Any>)>,
    mut on_dirty: impl FnMut(RoutingPath),
) {
    for (path, message) in messages {
        let mut ctx = MessageCtx::new(message);

        root_view.dispatch(root_element, path.as_slice(), Dispatch::Message(&mut ctx));

        if ctx.rebuild_requested() {
            on_dirty(path);
        }
    }
}

pub fn rebuild_dirty(
    driver: &Rc<dyn Driver>,
    event_tx: &mpsc::Sender<()>,
    provide_scope: &ProvideScope,
    root_element: &mut Element,
    root_view: &impl View,
    dirty: impl ExactSizeIterator<Item = RoutingPath>,
) {
    let n = dirty.len();
    let mut rebuilt: Vec<RoutingPath> = Vec::with_capacity(n);

    for path in dirty {
        let slice = path.as_slice();

        let mut routing_path = path.to_vec();

        root_view.dispatch(
            root_element,
            slice,
            Dispatch::Rebuild(&mut UpdateCtx::new(
                driver,
                event_tx,
                &mut routing_path,
                provide_scope,
            )),
        );

        rebuilt.push(path);
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc, sync::mpsc};

    use crate::{
        context::{Dispatch, UpdateCtx},
        driver::{dispatch_messages, rebuild_dirty, Driver},
        element::Element,
        provide::ProvideScope,
        render_object::RenderLeaf,
        routing_id::{RoutingId, RoutingPath},
        view::View,
    };

    struct NoopDriver;
    impl Driver for NoopDriver {}

    struct Recorder {
        message_calls: Cell<usize>,
        rebuild_calls: Cell<usize>,
        request_rebuild_on_message: bool,
    }

    impl Recorder {
        fn new() -> Self {
            Self {
                message_calls: Cell::new(0),
                rebuild_calls: Cell::new(0),
                request_rebuild_on_message: false,
            }
        }

        fn requesting_rebuild() -> Self {
            Self {
                request_rebuild_on_message: true,
                ..Self::new()
            }
        }
    }

    impl View for Recorder {
        type Render = RenderLeaf;
        type State = ();

        fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (Vec::new(), ())
        }

        fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

        fn dispatch(&self, _: &mut Element, path: &[RoutingId], action: Dispatch) {
            debug_assert!(path.is_empty(), "Recorder is a leaf");
            if !path.is_empty() {
                return;
            }
            match action {
                Dispatch::Message(ctx) => {
                    self.message_calls.set(self.message_calls.get() + 1);
                    let _: u32 = ctx.consume();
                    if self.request_rebuild_on_message {
                        ctx.request_rebuild();
                    }
                }
                Dispatch::Rebuild(_) => {
                    self.rebuild_calls.set(self.rebuild_calls.get() + 1);
                }
            }
        }

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    struct Transparent<Child> {
        child: Child,
    }

    impl<Child: View> View for Transparent<Child> {
        type Render = RenderLeaf;
        type State = ();

        fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (vec![Element::new(&self.child, ctx)], ())
        }

        fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
            element.child_mut(0, &old.child).update(&self.child, ctx);
        }

        fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
            element.child_mut(0, &self.child).dispatch(path, action)
        }

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    struct MultiChild<Child> {
        children: Vec<Child>,
    }

    impl<Child: View> View for MultiChild<Child> {
        type Render = RenderLeaf;
        type State = ();

        fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            let children = self
                .children
                .iter()
                .enumerate()
                .map(|(idx, child)| {
                    ctx.with_routing_id(RoutingId::new(idx as u16), |ctx| Element::new(child, ctx))
                })
                .collect();
            (children, ())
        }

        fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

        fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
            let Some((head, rest)) = path.split_first() else {
                return;
            };
            let idx = head.get() as usize;
            element
                .child_mut(idx, &self.children[idx])
                .dispatch(rest, action)
        }

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    #[test]
    fn message_dirties_target_and_rebuild_only_reaches_that_target() {
        //   MultiChild
        //   ├─ [0] Transparent -> Recorder (no rebuild)
        //   ├─ [1] Transparent -> Recorder (requests rebuild)
        //   └─ [2] Transparent -> Recorder (no rebuild)
        let view = MultiChild {
            children: vec![
                Transparent {
                    child: Recorder::new(),
                },
                Transparent {
                    child: Recorder::requesting_rebuild(),
                },
                Transparent {
                    child: Recorder::new(),
                },
            ],
        };

        let driver: Rc<dyn Driver> = Rc::new(NoopDriver);
        let (event_tx, _event_rx) = mpsc::channel();
        let provide_scope = ProvideScope::new();

        let mut routing_path = Vec::new();
        let mut root = Element::new(
            &view,
            &mut UpdateCtx::new(&driver, &event_tx, &mut routing_path, &provide_scope),
        );

        let target_path: RoutingPath = vec![RoutingId::new(1)].into();
        let messages = vec![(target_path, Box::new(42_u32) as Box<dyn std::any::Any>)];

        let mut dirty: Vec<RoutingPath> = Vec::new();
        dispatch_messages(
            &mut root,
            &view,
            messages.into_iter(),
            |path| dirty.push(path),
        );

        assert_eq!(dirty.len(), 1, "exactly one element should be dirtied");
        assert_eq!(dirty[0].as_slice(), &[RoutingId::new(1)]);

        rebuild_dirty(
            &driver,
            &event_tx,
            &provide_scope,
            &mut root,
            &view,
            dirty.into_iter(),
        );

        assert_eq!(view.children[0].child.rebuild_calls.get(), 0);
        assert_eq!(view.children[1].child.rebuild_calls.get(), 1);
        assert_eq!(view.children[2].child.rebuild_calls.get(), 0);
    }
}

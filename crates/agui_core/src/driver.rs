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
        context::UpdateCtx,
        driver::{Driver, dispatch_messages, rebuild_dirty},
        element::Element,
        provide::ProvideScope,
        routing_id::{RoutingId, RoutingPath},
        test_fixtures::{Leaf, MultiChild, Transparent},
    };

    struct NoopDriver;
    impl Driver for NoopDriver {}

    #[test]
    fn message_dirties_target_and_rebuild_only_reaches_that_target() {
        //   MultiChild
        //   ├─ [0] Transparent -> Leaf (no rebuild)
        //   ├─ [1] Transparent -> Leaf (requests rebuild)
        //   └─ [2] Transparent -> Leaf (no rebuild)
        let r0 = Cell::new(0_usize);
        let r1 = Cell::new(0_usize);
        let r2 = Cell::new(0_usize);

        let view = MultiChild {
            children: vec![
                Transparent {
                    child: Leaf::new().on_rebuild(|_| r0.set(r0.get() + 1)),
                },
                Transparent {
                    child: Leaf::new()
                        .on_message(|ctx| ctx.request_rebuild())
                        .on_rebuild(|_| r1.set(r1.get() + 1)),
                },
                Transparent {
                    child: Leaf::new().on_rebuild(|_| r2.set(r2.get() + 1)),
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
        dispatch_messages(&mut root, &view, messages.into_iter(), |path| {
            dirty.push(path)
        });

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

        assert_eq!(r0.get(), 0);
        assert_eq!(r1.get(), 1);
        assert_eq!(r2.get(), 0);
    }
}

use std::{rc::Rc, sync::mpsc};

use fnv::FnvHashSet;

use crate::{
    context::{Dispatch, UpdateCtx},
    element::Element,
    provide::ProvideScope,
    routing_id::RoutingPath,
    view::View,
};

pub trait Driver {}

pub fn rebuild_dirty(
    driver: &Rc<dyn Driver>,
    event_tx: &mpsc::Sender<()>,
    provide_scope: &ProvideScope,
    root_element: &mut Element,
    root_view: &impl View,
    dirty: impl ExactSizeIterator<Item = RoutingPath>,
) {
    let n = dirty.len();

    // Promotion threshold is `16 * depth`; with depth ≥ 1, n ≤ 16 can never trigger
    // it, so skip the `Option<FnvHashSet>` machinery entirely on small batches.
    if n <= 16 {
        rebuild_dirty_linear(
            driver,
            event_tx,
            provide_scope,
            root_element,
            root_view,
            dirty,
        );
    } else {
        rebuild_dirty_adaptive(
            driver,
            event_tx,
            provide_scope,
            root_element,
            root_view,
            dirty,
        );
    }
}

fn rebuild_dirty_linear(
    driver: &Rc<dyn Driver>,
    event_tx: &mpsc::Sender<()>,
    provide_scope: &ProvideScope,
    root_element: &mut Element,
    root_view: &impl View,
    dirty: impl ExactSizeIterator<Item = RoutingPath>,
) {
    let n = dirty.len();
    let mut rebuilt: Vec<RoutingPath> = Vec::with_capacity(n);
    let mut prev_depth = 0;

    for path in dirty {
        let slice = path.as_slice();
        let depth = slice.len();

        debug_assert!(depth >= prev_depth, "dirty paths must be sorted by length");

        prev_depth = depth;

        if rebuilt.iter().any(|r| path.is_descendant_of(r)) {
            continue;
        }

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

#[cold]
fn rebuild_dirty_adaptive(
    driver: &Rc<dyn Driver>,
    event_tx: &mpsc::Sender<()>,
    provide_scope: &ProvideScope,
    root_element: &mut Element,
    root_view: &impl View,
    dirty: impl ExactSizeIterator<Item = RoutingPath>,
) {
    let n = dirty.len();
    let mut rebuilt: Vec<RoutingPath> = Vec::with_capacity(n);
    let mut prev_depth = 0;

    let mut rebuilt_set: Option<FnvHashSet<RoutingPath>> = None;

    for path in dirty {
        let slice = path.as_slice();
        let depth = slice.len();

        debug_assert!(depth >= prev_depth, "dirty paths must be sorted by length");

        prev_depth = depth;

        // Dispatch is recursive: rebuilding an ancestor transitively rebuilds its
        // descendants, so any queued descendant path can be skipped.
        let dominated = match &rebuilt_set {
            Some(set) => (1..depth).any(|i| set.contains(&slice[..i])),
            None => rebuilt.iter().any(|r| path.is_descendant_of(r)),
        };

        if dominated {
            continue;
        }

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

        if let Some(set) = &mut rebuilt_set {
            set.insert(path);
        } else {
            rebuilt.push(path);

            if rebuilt.len() > 16 * depth.max(1) {
                rebuilt_set = Some(rebuilt.iter().cloned().collect());
            }
        }
    }
}

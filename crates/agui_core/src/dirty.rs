//! A set of dirty node handles, the reactivity engine's scheduling primitive. A driver marks handles as
//! work is requested, then drains them shallowest-first so a parent is processed before any child it might
//! reconcile. The set lives with the driver, not on the [`Tree`], so each pass keeps its own.

use crate::tree::{NodeDispatch, NodeHandle, Tree};

/// A set of node handles awaiting work.
#[derive(Default)]
pub struct Dirty {
    handles: Vec<NodeHandle>,
}

impl Dirty {
    /// An empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether nothing is queued.
    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }

    /// Queues `handle`. A handle already queued is not queued again.
    pub fn mark(&mut self, handle: NodeHandle) {
        if !self.handles.contains(&handle) {
            self.handles.push(handle);
        }
    }

    /// Removes and returns the shallowest queued handle whose node is still in `tree`, dropping any whose
    /// node has since been removed. Returns `None` once nothing live remains; a handle queued during a drain
    /// is honored.
    pub fn take_shallowest<R, N>(&mut self, tree: &Tree<R, N>) -> Option<NodeHandle>
    where
        N: NodeDispatch,
    {
        let shallowest = self
            .handles
            .iter()
            .enumerate()
            .filter_map(|(index, &handle)| tree.depth(handle).map(|depth| (depth, index)))
            .min_by_key(|&(depth, _)| depth)
            .map(|(_, index)| index);

        if let Some(index) = shallowest {
            Some(self.handles.swap_remove(index))
        } else {
            // No queued handle resolves, so every one left is gone; drop them all.
            self.handles.clear();
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ptr::NonNull;

    use crate::tree::{Cursor, NodeDispatch, Slot, Tree};

    use super::Dirty;

    // A minimal consumer: mount threads a cursor; these tests never dispatch, so the glue is never called.
    struct App;

    impl NodeDispatch for App {
        type Operation<'a> = ();
    }

    trait Element {
        fn mount(&mut self, ctx: MountCtx<'_>) {
            let _ = ctx;
        }
    }

    unsafe fn run<C: Element>(_data: NonNull<()>, _operation: ()) {}

    struct MountCtx<'a> {
        cursor: Cursor<'a, App>,
    }
    impl MountCtx<'_> {
        fn child<C: Element>(&mut self, slot: &mut Slot<C>) {
            // SAFETY: `slot` is the caller's field, `run::<C>` dispatches it, and the tree is only dropped,
            // so the handle is never used after teardown.
            let cursor = unsafe { self.cursor.register(slot, run::<C>) };
            slot.get_mut().mount(MountCtx { cursor });
        }
    }

    struct Leaf;
    impl Element for Leaf {}

    struct Mid {
        leaf: Slot<Leaf>,
    }
    impl Element for Mid {
        fn mount(&mut self, mut ctx: MountCtx<'_>) {
            ctx.child(&mut self.leaf);
        }
    }

    struct Root {
        mid: Slot<Mid>,
    }
    impl Element for Root {
        fn mount(&mut self, mut ctx: MountCtx<'_>) {
            ctx.child(&mut self.mid);
        }
    }

    fn tree() -> Tree<Root, App> {
        Tree::<Root, App>::new(
            Root {
                mid: Slot::new(Mid {
                    leaf: Slot::new(Leaf),
                }),
            },
            run::<Root>,
            |root, cursor| root.mount(MountCtx { cursor }),
        )
    }

    #[test]
    fn drains_shallowest_first_regardless_of_mark_order() {
        let tree = tree();
        let root = tree.root_handle();
        let mid = tree.root().mid.handle();
        let leaf = tree.root().mid.get().leaf.handle();

        let mut dirty = Dirty::new();
        dirty.mark(leaf);
        dirty.mark(root);
        dirty.mark(mid);

        assert_eq!(dirty.take_shallowest(&tree), Some(root));
        assert_eq!(dirty.take_shallowest(&tree), Some(mid));
        assert_eq!(dirty.take_shallowest(&tree), Some(leaf));
        assert!(dirty.take_shallowest(&tree).is_none());
        assert!(dirty.is_empty());
    }

    #[test]
    fn marking_twice_drains_once() {
        let tree = tree();
        let root = tree.root_handle();

        let mut dirty = Dirty::new();
        dirty.mark(root);
        dirty.mark(root);

        assert_eq!(dirty.take_shallowest(&tree), Some(root));
        assert!(dirty.take_shallowest(&tree).is_none());
    }
}

//! `unmount` recursively deregisters a subtree before its bodies are freed, so no `Head` is left
//! pointing at freed memory. Exercises both child kinds (inline `Slot` and dynamic `insert_child`) and
//! a nested teardown, checked here under a plain run and under miri for use-after-free.

use std::ptr::NonNull;

use agui_core::tree::{
    BoxedSlot, Mounter, Node, NodeHandle, Operation, Slot, Tree, TriggerCtx, Unmounter,
};

#[derive(Clone, Copy)]
enum Op {
    Build,
    Teardown,
}

impl Operation for Op {
    type Op<'a> = Op;
}

/// Owns a dynamically inserted child by raw pointer, freeing it on drop. The owner deregisters the
/// child's handle (with `unmount`) before it is dropped, so the box is freed only after its `Head` is
/// gone.
struct Dyn<C> {
    ptr: NonNull<C>,
    handle: NodeHandle,
}

impl<C> Drop for Dyn<C> {
    fn drop(&mut self) {
        // SAFETY: `ptr` came from `Box::into_raw` where the child was inserted, and is freed once here.
        unsafe { drop(Box::from_raw(self.ptr.as_ptr())) };
    }
}

struct Leaf;
impl Node<Op> for Leaf {
    fn mount(&mut self, _mounter: &mut Mounter<'_, Self, Op>) {}
    fn unmount(&mut self, _unmounter: &mut Unmounter<'_, Op>) {}
    fn trigger(&mut self, _cx: &mut TriggerCtx<'_, Op>, _op: Op) {}
}

struct Twig;
impl Node<Op> for Twig {
    fn mount(&mut self, _mounter: &mut Mounter<'_, Self, Op>) {}
    fn unmount(&mut self, _unmounter: &mut Unmounter<'_, Op>) {}
    fn trigger(&mut self, _cx: &mut TriggerCtx<'_, Op>, _op: Op) {}
}

/// Has one inline child (`leaf`) and one dynamic child (`twig`, added on `Build`).
struct Mid {
    leaf: Slot<Leaf>,
    twig: Option<Dyn<Twig>>,
}

impl Node<Op> for Mid {
    fn mount(&mut self, mounter: &mut Mounter<'_, Self, Op>) {
        // SAFETY: `leaf` is a field, and `unmount` deregisters it.
        unsafe { mounter.slot(&mut self.leaf) };
    }

    fn unmount(&mut self, unmounter: &mut Unmounter<'_, Op>) {
        unmounter.slot(&mut self.leaf);
        if let Some(twig) = &mut self.twig {
            // SAFETY: `twig.ptr` is the live `Twig` for `twig.handle`; its box is freed only when this
            // `Mid` drops, after this unmount returns.
            unsafe { unmounter.child::<Twig>(twig.ptr, twig.handle) };
        }
    }

    fn trigger(&mut self, cx: &mut TriggerCtx<'_, Op>, op: Op) {
        if let Op::Build = op {
            // SAFETY: `Box::into_raw` is never null.
            let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(Twig))) };
            // SAFETY: `ptr` is a fresh, stable allocation owned by the `Dyn` below until unmounted.
            let handle = unsafe { cx.insert_child::<Twig>(ptr) };
            self.twig = Some(Dyn { ptr, handle });
        }
    }
}

struct Root {
    mid: Option<Dyn<Mid>>,
}

impl Node<Op> for Root {
    fn mount(&mut self, _mounter: &mut Mounter<'_, Self, Op>) {}

    fn unmount(&mut self, unmounter: &mut Unmounter<'_, Op>) {
        if let Some(mid) = &mut self.mid {
            // SAFETY: `mid.ptr` is the live `Mid` for `mid.handle`; its body is freed by the owning
            // `Dyn`'s drop, after this unmount returns.
            unsafe { unmounter.child::<Mid>(mid.ptr, mid.handle) };
        }
    }

    fn trigger(&mut self, cx: &mut TriggerCtx<'_, Op>, op: Op) {
        match op {
            Op::Build => {
                let mid = Box::new(Mid {
                    leaf: Slot::new(Leaf),
                    twig: None,
                });
                // SAFETY: `Box::into_raw` is never null.
                let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(mid)) };
                // SAFETY: `ptr` is a fresh, stable allocation owned by the `Dyn` below until unmounted.
                let handle = unsafe { cx.insert_child::<Mid>(ptr) };
                self.mid = Some(Dyn { ptr, handle });
            }

            Op::Teardown => {
                if let Some(mid) = self.mid.take() {
                    // SAFETY: `mid.ptr` is the live `Mid` for `mid.handle`; its box is freed by the
                    // `drop(mid)` below, after this unmount returns.
                    unsafe { cx.unmounter().child::<Mid>(mid.ptr, mid.handle) };
                    drop(mid);
                }
            }
        }
    }
}

#[test]
fn unmount_deregisters_the_whole_subtree() {
    let mut tree: Tree<Root, Op> = Tree::new(Root { mid: None });
    let root = tree.root_handle();

    // Build: root inserts Mid (whose mount registers the inline Leaf), then Mid inserts Twig.
    tree.trigger(root, Op::Build);
    let mid_ptr = tree.root_node().mid.as_ref().unwrap().ptr;
    let mid = tree.root_node().mid.as_ref().unwrap().handle;
    // SAFETY: the Mid box is live (owned by `root.mid`); reading its inline leaf handle is fine.
    let leaf = unsafe { (*mid_ptr.as_ptr()).leaf.handle() };

    tree.trigger(mid, Op::Build);
    // SAFETY: the Mid box is still live; its twig was just inserted.
    let twig = unsafe { (*mid_ptr.as_ptr()).twig.as_ref().unwrap().handle };

    // The whole subtree is registered.
    assert!(tree.depth(mid).is_some());
    assert!(tree.depth(leaf).is_some());
    assert!(tree.depth(twig).is_some());

    // Teardown: root unmounts the Mid subtree (recursively deregistering Leaf and Twig) and frees it.
    tree.trigger(root, Op::Teardown);

    // Every handle in the torn-down subtree is gone; triggering them is a no-op, not a freed-memory read.
    assert!(tree.depth(mid).is_none());
    assert!(tree.depth(leaf).is_none());
    assert!(tree.depth(twig).is_none());
    assert!(!tree.trigger(mid, Op::Build));
    assert!(!tree.trigger(leaf, Op::Build));
    assert!(!tree.trigger(twig, Op::Build));

    // The root is untouched.
    assert_eq!(tree.depth(root), Some(0));
}

#[test]
fn rebuild_after_teardown_reuses_the_tree() {
    let mut tree: Tree<Root, Op> = Tree::new(Root { mid: None });
    let root = tree.root_handle();

    tree.trigger(root, Op::Build);
    let first = tree.root_node().mid.as_ref().unwrap().handle;
    tree.trigger(root, Op::Teardown);

    // A fresh build after a full teardown registers a new subtree the tree can drive again.
    tree.trigger(root, Op::Build);
    let second = tree.root_node().mid.as_ref().unwrap().handle;
    assert_ne!(first, second, "the rebuilt child gets a fresh handle");
    assert!(tree.depth(second).is_some());
    assert!(!tree.trigger(first, Op::Build), "the old handle stays dead");
}

/// Owns one erased dynamic child through [`BoxedSlot`], built on `Build` and torn down on `Teardown`. The
/// concrete child type (`Leaf`) is not named in the field.
struct Host {
    child: Option<BoxedSlot<Op>>,
}

impl Node<Op> for Host {
    fn mount(&mut self, _mounter: &mut Mounter<'_, Self, Op>) {}

    fn unmount(&mut self, unmounter: &mut Unmounter<'_, Op>) {
        if let Some(child) = &mut self.child {
            unmounter.boxed_slot(child);
        }
    }

    fn trigger(&mut self, cx: &mut TriggerCtx<'_, Op>, op: Op) {
        match op {
            // SAFETY: `Host::unmount` (and `Op::Teardown`) deregister this child before it drops, and the
            // test builds it only once, so it is never overwritten while still registered.
            Op::Build => self.child = Some(unsafe { BoxedSlot::new(Leaf, cx) }),
            Op::Teardown => {
                if let Some(mut child) = self.child.take() {
                    cx.unmounter().boxed_slot(&mut child);
                }
            }
        }
    }
}

#[test]
fn boxed_erases_a_dynamic_child_and_unmounts_it() {
    let mut tree: Tree<Host, Op> = Tree::new(Host { child: None });
    let root = tree.root_handle();

    tree.trigger(root, Op::Build);
    let child = tree.root_node().child.as_ref().unwrap().handle();
    assert!(tree.depth(child).is_some());

    // The concrete type is recoverable through the erased wrapper.
    assert!(
        tree.root_node()
            .child
            .as_ref()
            .unwrap()
            .as_any()
            .is::<Leaf>()
    );

    tree.trigger(root, Op::Teardown);
    assert!(tree.depth(child).is_none());
    assert!(
        !tree.trigger(child, Op::Build),
        "the torn-down handle is dead"
    );
}

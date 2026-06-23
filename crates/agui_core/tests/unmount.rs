//! Teardown deregisters a subtree before its bodies are freed, so no registry entry dangles. Exercised
//! here under a plain run and under miri for use-after-free. Being a separate crate, this also proves the
//! consumer pattern is self-contained: the tree names no node trait, so elements impl a crate-local trait
//! (`Element`) and hand the tree a dispatch fn, with no wiring back to core.

use std::ptr::NonNull;

use agui_core::tree::{BoxedSlot, Cursor, NodeDispatch, Slot, Tree};

struct App;

impl NodeDispatch for App {
    type Operation<'a> = Operation<'a>;
}

/// The consumer's local element trait. Mount and unmount are concrete lifecycle methods called directly;
/// `build` and `message` are reached through the tree, the glue unwrapping [`Operation`] onto them.
trait Element {
    fn mount(&mut self, ctx: MountCtx<'_>) {
        let _ = ctx;
    }
    fn unmount(&mut self, ctx: UnmountCtx<'_>) {
        let _ = ctx;
    }
    fn build(&mut self, ctx: BuildCtx<'_>) {
        let _ = ctx;
    }
    fn message(&mut self) {}
}

/// The dispatch fn the consumer hands the tree for a `C`: unwraps the operation onto the matching method.
unsafe fn run<C: Element>(data: NonNull<()>, operation: Operation<'_>) {
    // SAFETY: only ever stored against a `data` that points at a live `C`.
    let node = unsafe { &mut *data.cast::<C>().as_ptr() };
    match operation {
        Operation::Build(ctx) => node.build(ctx),
        Operation::Message => node.message(),
    }
}

struct MountCtx<'a> {
    cursor: Cursor<'a, App>,
}
struct UnmountCtx<'a> {
    cursor: Cursor<'a, App>,
}
struct BuildCtx<'a> {
    cursor: Cursor<'a, App>,
    target: usize,
}
enum Operation<'a> {
    Build(BuildCtx<'a>),
    Message,
}

impl MountCtx<'_> {
    fn child<C: Element>(&mut self, slot: &mut Slot<C>) {
        // SAFETY: `slot` is the caller's field, `run::<C>` dispatches it, and its unmount deregisters it.
        let cursor = unsafe { self.cursor.register(slot, run::<C>) };
        slot.get_mut().mount(MountCtx { cursor });
    }
}
impl UnmountCtx<'_> {
    fn child<C: Element>(&mut self, slot: &mut Slot<C>) {
        // SAFETY: `slot` is this element's own child.
        unsafe {
            self.cursor
                .with_child(slot, |child, cursor| child.unmount(UnmountCtx { cursor }));
        }

        self.cursor.deregister(slot);
    }

    fn boxed<C: Element>(&mut self, child: &mut BoxedSlot<C>) {
        // SAFETY: `child` is this element's own child.
        unsafe {
            self.cursor.with_child(child, |element, cursor| {
                element.unmount(UnmountCtx { cursor });
            });
        }

        self.cursor.deregister(child);
    }
}
impl BuildCtx<'_> {
    fn insert<C: Element>(&mut self, child: C) -> BoxedSlot<C> {
        let mut owner = BoxedSlot::new(child);
        // SAFETY: `run::<C>` dispatches the child, and the caller deregisters it before drop.
        let cursor = unsafe { self.cursor.register(&mut owner, run::<C>) };
        owner.get_mut().mount(MountCtx { cursor });
        owner
    }
    fn remove<C: Element>(&mut self, mut child: BoxedSlot<C>) {
        // SAFETY: `child` is this element's own child.
        unsafe {
            self.cursor.with_child(&mut child, |element, cursor| {
                element.unmount(UnmountCtx { cursor });
            });
        }

        self.cursor.deregister(&mut child);
    }
}

// A leaf with one inline child of its own, so a removed subtree is two levels deep.
struct Leaf;
impl Element for Leaf {}

struct Mid {
    leaf: Slot<Leaf>,
}
impl Element for Mid {
    fn mount(&mut self, mut ctx: MountCtx<'_>) {
        ctx.child(&mut self.leaf);
    }
    fn unmount(&mut self, mut ctx: UnmountCtx<'_>) {
        ctx.child(&mut self.leaf);
    }
}

// The root grows/shrinks dynamic `Mid` children toward `target` during build; each `Mid` carries an
// inline `Leaf`, so removing a `Mid` tears down a whole subtree.
struct Root {
    kids: Vec<BoxedSlot<Mid>>,
}
impl Element for Root {
    fn unmount(&mut self, mut ctx: UnmountCtx<'_>) {
        for kid in &mut self.kids {
            ctx.boxed(kid);
        }
    }
    fn build(&mut self, mut ctx: BuildCtx<'_>) {
        while self.kids.len() < ctx.target {
            self.kids.push(ctx.insert(Mid {
                leaf: Slot::new(Leaf),
            }));
        }
        while self.kids.len() > ctx.target {
            let kid = self.kids.pop().unwrap();
            ctx.remove(kid);
        }
    }
}

fn built(target: usize) -> Tree<Root, App> {
    let mut tree =
        Tree::<Root, App>::new(Root { kids: Vec::new() }, run::<Root>, |root, cursor| {
            root.mount(MountCtx { cursor })
        });
    let root = tree.root_handle();
    tree.dispatch_with_cursor(root, |cursor| Operation::Build(BuildCtx { cursor, target }));
    tree
}

#[test]
fn build_constructs_a_deep_subtree() {
    let tree = built(2);
    assert_eq!(tree.root().kids.len(), 2);
    for kid in &tree.root().kids {
        assert_eq!(tree.depth(kid.handle()), Some(1));
        assert_eq!(tree.depth(kid.get().leaf.handle()), Some(2));
    }
}

#[test]
fn removing_a_child_tears_down_its_whole_subtree() {
    let mut tree = built(2);
    let root = tree.root_handle();
    let mid = tree.root().kids[1].handle();
    let leaf = tree.root().kids[1].get().leaf.handle();

    // Shrink to 1: the popped `Mid` is unmounted, recursively deregistering its inline `Leaf`.
    tree.dispatch_with_cursor(root, |cursor| {
        Operation::Build(BuildCtx { cursor, target: 1 })
    });
    assert_eq!(tree.root().kids.len(), 1);

    assert!(tree.depth(mid).is_none(), "the removed mid is gone");
    assert!(tree.depth(leaf).is_none(), "its inline leaf is gone too");
    assert!(
        !tree.dispatch(mid, Operation::Message),
        "the removed handle is dead"
    );
    assert!(!tree.dispatch(leaf, Operation::Message));
}

#[test]
fn dropping_the_tree_frees_the_whole_subtree() {
    // Just exercising Drop on a populated tree; miri's leak/UAF checks do the verifying.
    let _tree = built(3);
}

//! A statically-typed tree of reactive nodes, addressed by copyable [`NodeHandle`]s.
//!
//! [`Tree`] keeps its root [`Node`] and that node's descendants inline in a single allocation, and
//! hands back a [`NodeHandle`] for each. You make a node do work by triggering it with some operation
//! `T`: the node receives the operation, acts on it, and can reach its own children along the way.
//! Looking a node up by handle, or triggering it, is O(1). [`Tree::depth`] reports how deep a handle
//! sits, so a caller draining a batch of them can trigger parents before children.
//!
//! A handle stays safe to use after its node is gone: [`Tree::trigger`] returns `false` instead of
//! touching freed memory. Most children live inline in the parent's allocation; one created on the fly,
//! which needs a stable address of its own, gets its own allocation and is registered with
//! [`TriggerCtx::insert_child`].
//!
//! # Example
//!
//! ```
//! use agui_core::tree::{Mounter, Node, Operation, Tree, TriggerCtx, Unmounter};
//!
//! // The operation that drives this tree: a `u32` added to a running total.
//! struct Add;
//! impl Operation for Add {
//!     type Op<'a> = u32;
//! }
//!
//! struct Counter(u32);
//!
//! impl Node<Add> for Counter {
//!     fn mount(&mut self, _mounter: &mut Mounter<'_, Self, Add>) {}
//!     fn unmount(&mut self, _unmounter: &mut Unmounter<'_, Add>) {}
//!     fn trigger(&mut self, _cx: &mut TriggerCtx<'_, Add>, op: u32) {
//!         self.0 += op;
//!     }
//! }
//!
//! let mut tree: Tree<Counter, Add> = Tree::new(Counter(0));
//! let root = tree.root_handle();
//!
//! tree.trigger(root, 5);
//!
//! assert_eq!(tree.root_node().0, 5);
//! ```

use std::any::Any;
use std::mem::offset_of;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

use slotmap::{Key, SlotMap, new_key_type};

new_key_type! {
    /// A small, copyable handle to a node. Good until that node is removed.
    pub struct NodeHandle;
}

/// The family of operations that drive a [`Tree`]: a lifetime-free type, so it can sit on `Tree<R, T>`
/// for the tree's whole life, while the operation a node actually receives, [`Op`](Self::Op), may borrow
/// for the trigger call only.
pub trait Operation: 'static {
    type Op<'a>;
}

/// A node in the [`Tree`]. It does its work when triggered with an operation `T`, and tells the tree
/// about its inline children when it mounts.
pub trait Node<T: Operation>: 'static {
    /// Tells the tree about this node's inline children.
    fn mount(&mut self, mounter: &mut Mounter<'_, Self, T>);

    /// The inverse of [`mount`](Self::mount): deregisters this node's children as its subtree is torn
    /// down. Mirror `mount`, calling [`Unmounter::slot`] for each inline child and [`Unmounter::child`]
    /// for each dynamic one. The default does nothing, for a node with no children.
    fn unmount(&mut self, unmounter: &mut Unmounter<'_, T>);

    /// Does the node's work for one trigger, driven by `op`.
    fn trigger(&mut self, cx: &mut TriggerCtx<'_, T>, op: T::Op<'_>);
}

/// The object-safe, type-erased form of [`Node`], letting a parent own a child whose concrete type it
/// does not name. A node is mounted and triggered through its concrete type, so the erased form only
/// needs to cover teardown and downcasting.
pub trait AnyNode<T: Operation> {
    /// The node as `&dyn Any`, for downcasting back to its concrete type.
    fn as_any(&self) -> &dyn Any;

    /// The node as `&mut dyn Any`, for downcasting back to its concrete type.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// The erased form of [`Node::unmount`].
    fn dyn_unmount(&mut self, unmounter: &mut Unmounter<'_, T>);
}

impl<N: Node<T>, T: Operation> AnyNode<T> for N {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn dyn_unmount(&mut self, unmounter: &mut Unmounter<'_, T>) {
        self.unmount(unmounter);
    }
}

/// Casts a type-erased node pointer back to `N` and triggers it. There's one of these per node type,
/// which is how the registry drives a node it only holds an untyped pointer to.
///
/// # Safety
/// `node` must point at a live value of type `N`.
unsafe fn run_glue<N: Node<T>, T: Operation>(
    node: NonNull<()>,
    cx: &mut TriggerCtx<'_, T>,
    op: T::Op<'_>,
) {
    // SAFETY: the caller guarantees `node` points at a live `N`, which this `Tree` uniquely accesses
    // for the duration of the trigger.
    unsafe { (*node.cast::<N>().as_ptr()).trigger(cx, op) };
}

/// What the registry keeps for one node: its depth, a raw pointer to the body, and the glue to trigger
/// it. The body lives elsewhere, inline in the tree or in its own allocation for a dynamic child.
struct Head<T: Operation> {
    depth: u32,
    node: NonNull<()>,
    run: unsafe fn(NonNull<()>, &mut TriggerCtx<'_, T>, T::Op<'_>),
}

/// The lookup from handle to node, owned by [`Tree`] and reached by a node only through its [`TriggerCtx`].
struct Registry<T: Operation> {
    heads: SlotMap<NodeHandle, Head<T>>,
}

impl<T: Operation> Registry<T> {
    fn new() -> Self {
        Registry {
            heads: SlotMap::with_key(),
        }
    }

    fn insert(
        &mut self,
        node: NonNull<()>,
        run: unsafe fn(NonNull<()>, &mut TriggerCtx<'_, T>, T::Op<'_>),
        depth: u32,
    ) -> NodeHandle {
        self.heads.insert(Head { depth, node, run })
    }

    fn remove(&mut self, handle: NodeHandle) {
        self.heads.remove(handle);
    }
}

/// Hands a mounting node a way to register its inline children. Each child's address is worked out from
/// the parent's base pointer, so the handle keeps pointing at the right place for as long as the parent
/// lives.
pub struct Mounter<'a, P: ?Sized, T: Operation> {
    registry: &'a mut Registry<T>,
    this: NonNull<P>,
    depth: u32,
}

impl<P: ?Sized, T: Operation> Mounter<'_, P, T> {
    /// Registers an inline child held in `slot`, mounts it, and records its handle in the slot.
    ///
    /// # Safety
    /// - `slot` must be one of the node's own fields, so it lies within the node's allocation (checked in
    ///   debug builds).
    /// - The node's [`unmount`](Node::unmount) must deregister this child with [`Unmounter::slot`];
    ///   otherwise its registry entry dangles once the subtree is torn down, and a later
    ///   [`Tree::trigger`] of the child's handle dereferences freed memory.
    ///
    /// # Example
    ///
    /// ```
    /// use agui_core::tree::{Mounter, Node, Operation, Slot, Tree, TriggerCtx, Unmounter};
    ///
    /// struct Reg;
    /// impl Operation for Reg {
    ///     type Op<'a> = u32;
    /// }
    ///
    /// struct Child {
    ///     hits: u32,
    /// }
    /// impl Child {
    ///     fn hits(&self) -> u32 {
    ///         self.hits
    ///     }
    /// }
    /// impl Node<Reg> for Child {
    ///     fn mount(&mut self, _mounter: &mut Mounter<'_, Self, Reg>) {}
    ///     fn unmount(&mut self, _unmounter: &mut Unmounter<'_, Reg>) {}
    ///     fn trigger(&mut self, _cx: &mut TriggerCtx<'_, Reg>, op: u32) {
    ///         self.hits += op;
    ///     }
    /// }
    ///
    /// struct Parent {
    ///     child: Slot<Child>,
    /// }
    /// impl Node<Reg> for Parent {
    ///     fn mount(&mut self, mounter: &mut Mounter<'_, Self, Reg>) {
    ///         // SAFETY: `child` is a field, and `unmount` below deregisters it.
    ///         unsafe { mounter.slot(&mut self.child) };
    ///     }
    ///     fn unmount(&mut self, unmounter: &mut Unmounter<'_, Reg>) {
    ///         unmounter.slot(&mut self.child);
    ///     }
    ///     fn trigger(&mut self, _cx: &mut TriggerCtx<'_, Reg>, _op: u32) {}
    /// }
    ///
    /// let mut tree: Tree<Parent, Reg> = Tree::new(Parent { child: Slot::new(Child { hits: 0 }) });
    /// let child = tree.root_node().child.handle();
    /// tree.trigger(child, 7);
    ///
    /// assert_eq!(tree.root_node().child.hits(), 7);
    /// ```
    pub unsafe fn slot<C: Node<T>>(&mut self, slot: &mut Slot<C>)
    where
        P: Sized,
    {
        let base = self.this.as_ptr() as usize;
        let offset = (std::ptr::from_mut(slot) as usize).wrapping_sub(base);
        debug_assert!(
            offset
                .checked_add(std::mem::size_of::<Slot<C>>())
                .is_some_and(|end| end <= std::mem::size_of::<P>()),
            "`slot` must be a field of the node being mounted",
        );

        let value_offset = offset + offset_of!(Slot<C>, value);

        // SAFETY: by the caller's contract `slot` is a field of this node, so `offset` is its offset
        // within the node and the body pointer lands on the child within the allocation. It carries the
        // node's provenance, is stored for the registry to form a `&mut` from at trigger time, and is not
        // dereferenced here: the mount below goes through `slot.value`, so this pointer stays at the
        // bottom of the borrow stack and the mount's writes leave it valid.
        let body = unsafe {
            NonNull::new_unchecked(self.this.as_ptr().byte_add(value_offset).cast::<C>())
        };

        let depth = self.depth + 1;
        slot.handle = self.registry.insert(body.cast(), run_glue::<C, T>, depth);

        let mut child = Mounter {
            registry: &mut *self.registry,
            this: body,
            depth,
        };

        slot.value.mount(&mut child);
    }
}

/// Deregisters a node's children as its subtree is torn down, the inverse of [`Mounter`]. Each method
/// recurses into the child's [`unmount`](Node::unmount) before removing the child's own registry entry,
/// so every handle in a subtree is gone by the time its bodies are freed.
pub struct Unmounter<'a, T: Operation> {
    registry: &'a mut Registry<T>,
}

impl<T: Operation> Unmounter<'_, T> {
    /// Deregisters an inline child held in a [`Slot`], and everything beneath it. The mirror of
    /// [`Mounter::slot`]; call it from [`Node::unmount`] for each child registered with `Mounter::slot`.
    pub fn slot<C: Node<T>>(&mut self, slot: &mut Slot<C>) {
        let mut child = Unmounter {
            registry: &mut *self.registry,
        };

        slot.value.unmount(&mut child);

        self.registry.remove(slot.handle);
        slot.handle = NodeHandle::null();
    }

    /// Deregisters a dynamically inserted child, and everything beneath it. The mirror of
    /// [`TriggerCtx::insert_child`]; the caller frees the child's body once this returns, by which point
    /// no handle into the subtree remains.
    ///
    /// # Safety
    /// `node` must point at the live `C` that `handle` was registered for.
    pub unsafe fn child<C: Node<T>>(&mut self, node: NonNull<C>, handle: NodeHandle) {
        let mut child = Unmounter {
            registry: &mut *self.registry,
        };

        // SAFETY: `node` is the live `C` for `handle` (the caller's contract); its body is freed only
        // after this returns, so forming the `&mut C` to recurse into is sound.
        unsafe { (*node.as_ptr()).unmount(&mut child) };

        self.registry.remove(handle);
    }

    /// Deregisters a [`BoxedSlot`] dynamic child, and everything beneath it. The mirror of [`BoxedSlot::new`];
    /// call it from [`Node::unmount`] for each [`BoxedSlot`] child. Nulls the child's handle, so the
    /// debug drop-check does not fire when it later drops.
    pub fn boxed_slot(&mut self, boxed: &mut BoxedSlot<T>) {
        let mut child = Unmounter {
            registry: &mut *self.registry,
        };

        // SAFETY: `boxed.node` is the live child for `boxed.handle`; its allocation is freed only when
        // `boxed` drops, after this returns.
        unsafe { (*boxed.node.as_ptr()).dyn_unmount(&mut child) };
        self.registry.remove(boxed.handle);
        boxed.handle = NodeHandle::null();
    }
}

/// An inline child node bundled with the [`NodeHandle`] the tree gives it, so a parent can hold a child
/// without tracking its handle by hand. Hold one in a field, register it from the parent's
/// [`mount`](Node::mount) with [`Mounter::slot`], and read the live handle back with
/// [`handle`](Self::handle). It derefs to the child, so the parent uses it as if it were the `C`.
pub struct Slot<C> {
    value: C,
    handle: NodeHandle,
}

impl<C> Slot<C> {
    /// Wraps `value` as a child that has not been mounted yet. Its handle is filled in when the parent
    /// registers it with [`Mounter::slot`]; until then [`handle`](Self::handle) is null.
    pub fn new(value: C) -> Self {
        Slot {
            value,
            handle: NodeHandle::default(),
        }
    }

    /// The handle the tree gave this child when it was mounted.
    pub fn handle(&self) -> NodeHandle {
        self.handle
    }
}

impl<C> Deref for Slot<C> {
    type Target = C;

    fn deref(&self) -> &C {
        &self.value
    }
}

impl<C> DerefMut for Slot<C> {
    fn deref_mut(&mut self) -> &mut C {
        &mut self.value
    }
}

// Debug-only: a `Slot` dropped while still registered (handle non-null) means the node's `unmount`
// forgot to deregister it, leaving a dangling `Head` and latent UB. Surface it as a panic. Skipped while
// already unwinding, so it never escalates an in-flight panic into an abort.
#[cfg(debug_assertions)]
impl<C> Drop for Slot<C> {
    fn drop(&mut self) {
        assert!(
            self.handle.is_null() || std::thread::panicking(),
            "a mounted `Slot` was dropped without being unmounted; \
             the node's `unmount` must call `Unmounter::slot`"
        );
    }
}

/// A dynamically inserted child that owns its own allocation, type-erased so a parent can hold children
/// of differing types uniformly. The heap analog of [`Slot`]: created on the fly during a trigger with
/// [`new`](Self::new), deregistered with [`Unmounter::boxed_slot`], and freed on drop. Derefs to the erased
/// [`AnyNode`] for downcasting back to a concrete type.
pub struct BoxedSlot<T: Operation> {
    node: NonNull<dyn AnyNode<T>>,
    handle: NodeHandle,
}

impl<T: Operation> BoxedSlot<T> {
    /// Boxes `node`, registers it as a dynamic child below the node being triggered, and returns the
    /// owning wrapper.
    ///
    /// # Safety
    /// The holding node's [`unmount`](Node::unmount) must deregister this with [`Unmounter::boxed_slot`];
    /// otherwise its registry entry dangles once the subtree is torn down, and a later [`Tree::trigger`]
    /// of its handle dereferences freed memory. The same obligation as [`Mounter::slot`].
    pub unsafe fn new<N: Node<T>>(node: N, cx: &mut TriggerCtx<'_, T>) -> Self {
        // SAFETY: `Box::into_raw` is never null.
        let node = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(node))) };
        // SAFETY: `node` is a fresh allocation owned by this `BoxedSlot` (freed on drop) and stays put until
        // unmounted, which satisfies `insert_child`.
        let handle = unsafe { cx.insert_child::<N>(node) };
        BoxedSlot { node, handle }
    }

    /// The handle the tree gave this child when it was inserted.
    pub fn handle(&self) -> NodeHandle {
        self.handle
    }
}

impl<T: Operation> Deref for BoxedSlot<T> {
    type Target = dyn AnyNode<T>;

    fn deref(&self) -> &Self::Target {
        // SAFETY: `node` is the live child this `BoxedSlot` owns; outside a trigger nothing holds a `&mut` to
        // it, so this shared borrow does not alias one.
        unsafe { self.node.as_ref() }
    }
}

impl<T: Operation> DerefMut for BoxedSlot<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: as `deref`, and `&mut self` rules out any other borrow of the child.
        unsafe { self.node.as_mut() }
    }
}

impl<T: Operation> Drop for BoxedSlot<T> {
    fn drop(&mut self) {
        // A `BoxedSlot` dropped while still registered (handle non-null) means the holding node's `unmount`
        // forgot to deregister it: a dangling `Head` and latent UB. Surface it as a debug panic, skipped
        // while already unwinding so it never escalates an in-flight panic into an abort.
        #[cfg(debug_assertions)]
        assert!(
            self.handle.is_null() || std::thread::panicking(),
            "a `BoxedSlot` child was dropped without being unmounted; \
             the holding node's `unmount` must call `Unmounter::boxed_slot`"
        );

        // SAFETY: `node` came from `Box::into_raw` in `new`, freed exactly once here.
        unsafe { drop(Box::from_raw(self.node.as_ptr())) };
    }
}

/// What a node can reach while it's being triggered: its own handle and depth, and the means to add or
/// remove children directly below it.
pub struct TriggerCtx<'a, T: Operation> {
    registry: &'a mut Registry<T>,
    this: NodeHandle,
    depth: u32,
}

impl<T: Operation> TriggerCtx<'_, T> {
    /// The handle of the node being triggered.
    pub fn this(&self) -> NodeHandle {
        self.this
    }

    /// The depth of the node being triggered.
    pub fn depth(&self) -> u32 {
        self.depth
    }

    /// Registers a child created on the fly, mounts it, and returns its handle.
    ///
    /// # Safety
    /// The `node` body must stay valid and at this address until it is unmounted. Tear the child down
    /// with [`unmounter`](Self::unmounter) *before* its body is freed: freeing a child while its handle,
    /// or any descendant's, is still registered is undefined behavior, because a later [`Tree::trigger`]
    /// of that handle dereferences freed memory.
    pub unsafe fn insert_child<C: Node<T>>(&mut self, node: NonNull<C>) -> NodeHandle {
        let depth = self.depth + 1;
        let handle = self.registry.insert(node.cast(), run_glue::<C, T>, depth);

        let mut mounter = Mounter {
            registry: &mut *self.registry,
            this: node,
            depth,
        };
        // SAFETY: `node` points at a live `C` in its own allocation (the caller's contract), so the
        // `&mut C` formed to mount it cannot alias the node currently being triggered.
        unsafe { (*node.as_ptr()).mount(&mut mounter) };

        handle
    }

    /// An [`Unmounter`] for tearing a child subtree down during this trigger: it deregisters a child and
    /// all its descendants, so the bodies can then be freed. Use it when a reconcile removes a child
    /// added with [`insert_child`](Self::insert_child).
    pub fn unmounter(&mut self) -> Unmounter<'_, T> {
        Unmounter {
            registry: &mut *self.registry,
        }
    }
}

/// A tree of reactive nodes, the root and its inline descendants together in one allocation, each
/// addressable by handle. Every node acts on the operation `T`. See the module docs.
pub struct Tree<R: Node<T>, T: Operation> {
    registry: Registry<T>,
    root: NonNull<R>,
    root_handle: NodeHandle,
}

/// Frees the root box if [`Tree::new`] unwinds before the `Tree` that will own it is built (say, a
/// node's `mount` panics). Forgotten once `new` succeeds.
struct UnmountedRoot<R>(NonNull<R>);

impl<R> Drop for UnmountedRoot<R> {
    fn drop(&mut self) {
        // SAFETY: until `Tree::new` returns, this guard is the sole owner of the box `into_raw` produced.
        unsafe { drop(Box::from_raw(self.0.as_ptr())) };
    }
}

impl<R: Node<T>, T: Operation> Tree<R, T> {
    /// Builds a tree from `root`, mounting it and everything inline beneath it.
    pub fn new(root: R) -> Self {
        let mut registry = Registry::<T>::new();
        // SAFETY: `Box::into_raw` never returns null.
        let base = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(root))) };
        let root_handle = registry.insert(base.cast(), run_glue::<R, T>, 0);

        // Free the root box if `mount` panics: nothing owns it until the `Tree` below is built.
        let guard = UnmountedRoot(base);
        let mut mounter = Mounter {
            registry: &mut registry,
            this: base,
            depth: 0,
        };
        // SAFETY: `base` points at the live root `R` just registered, and nothing else accesses it
        // while it mounts.
        unsafe { (*base.as_ptr()).mount(&mut mounter) };
        std::mem::forget(guard);

        Tree {
            registry,
            root: base,
            root_handle,
        }
    }

    /// The handle of the root node.
    pub fn root_handle(&self) -> NodeHandle {
        self.root_handle
    }

    /// Borrows the root node for reading.
    pub fn root_node(&self) -> &R {
        // SAFETY: `root` came from `Box::into_raw` of an `R` owned by this `Tree`, and `&self` rules out
        // any `&mut` to it (a trigger takes `&mut self`), so this shared read does not alias one.
        unsafe { &*self.root.as_ptr() }
    }

    /// How deep the node `handle` names sits, or `None` if it's gone. Order handles by this to trigger a
    /// parent before the children it might reconcile.
    pub fn depth(&self, handle: NodeHandle) -> Option<u32> {
        self.registry.heads.get(handle).map(|head| head.depth)
    }

    /// Operations the node `handle` names with `op`. Returns `false` if that node is already gone, in
    /// which case nothing happens.
    pub fn trigger(&mut self, handle: NodeHandle, op: T::Op<'_>) -> bool {
        let Some(head) = self.registry.heads.get(handle) else {
            return false;
        };

        let (node, run) = (head.node, head.run);
        let depth = head.depth;

        let mut cx = TriggerCtx {
            registry: &mut self.registry,
            this: handle,
            depth,
        };

        // SAFETY: `node` and `run` were just read from the same live `Head`; `run` is the glue
        // monomorphized for that node's type, and nothing else accesses the node during the call.
        unsafe { run(node, &mut cx, op) };

        true
    }
}

impl<R: Node<T>, T: Operation> Drop for Tree<R, T> {
    fn drop(&mut self) {
        // Tear the subtree down the way a removed child would, so every descendant `Slot` is deregistered
        // (and its handle nulled) before its body is freed.
        let mut unmounter = Unmounter {
            registry: &mut self.registry,
        };
        // SAFETY: `root` is the live root this tree owns; `&mut self` rules out any other access, and it
        // is freed just below.
        unsafe { (*self.root.as_ptr()).unmount(&mut unmounter) };

        // SAFETY: `root` came from `Box::into_raw` in `new` and is freed exactly once, here.
        unsafe { drop(Box::from_raw(self.root.as_ptr())) };
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use crate::tree::Unmounter;

    use super::{Mounter, Node, NodeHandle, NonNull, Operation, Slot, Tree, TriggerCtx};

    /// A pass's dirty list, handed to every trigger as the operation. The driver owns it, not the tree;
    /// a node reaches it to queue follow-up work, and the driver drains it rootmost-first.
    #[derive(Default)]
    struct DirtyList {
        dirty: Vec<NodeHandle>,
    }

    impl DirtyList {
        fn mark(&mut self, handle: NodeHandle) {
            if !self.dirty.contains(&handle) {
                self.dirty.push(handle);
            }
        }

        fn flush<R: Node<DirtyList>>(&mut self, tree: &mut Tree<R, DirtyList>) {
            while !self.dirty.is_empty() {
                self.dirty
                    .sort_by_key(|&handle| tree.depth(handle).unwrap_or(u32::MAX));

                let handle = self.dirty.remove(0);
                tree.trigger(handle, self);
            }
        }
    }

    impl Operation for DirtyList {
        type Op<'a> = &'a mut DirtyList;
    }

    #[derive(Clone, Default)]
    struct Log {
        order: Rc<RefCell<Vec<u32>>>,
        freed: Rc<Cell<u32>>,
    }

    struct Header {
        value: u32,
        log: Log,
    }

    impl Node<DirtyList> for Header {
        fn mount(&mut self, _mounter: &mut Mounter<'_, Self, DirtyList>) {}

        fn unmount(&mut self, _unmounter: &mut Unmounter<'_, DirtyList>) {}

        fn trigger(&mut self, _cx: &mut TriggerCtx<'_, DirtyList>, _op: &mut DirtyList) {
            self.log.order.borrow_mut().push(1);
            self.value += 1;
        }
    }

    struct Item {
        id: u32,
        log: Log,
    }

    impl Node<DirtyList> for Item {
        fn mount(&mut self, _mounter: &mut Mounter<'_, Self, DirtyList>) {}

        fn unmount(&mut self, _unmounter: &mut Unmounter<'_, DirtyList>) {}

        fn trigger(&mut self, _cx: &mut TriggerCtx<'_, DirtyList>, _op: &mut DirtyList) {
            self.log.order.borrow_mut().push(self.id);
        }
    }

    impl Drop for Item {
        fn drop(&mut self) {
            self.log.freed.set(self.log.freed.get() + 1);
        }
    }

    /// Holds a child made on the fly by raw pointer, and frees it on drop.
    struct DynItem {
        ptr: NonNull<Item>,
        handle: NodeHandle,
    }

    impl Drop for DynItem {
        fn drop(&mut self) {
            // SAFETY: `ptr` came from `Box::into_raw` in `App::trigger` and is freed exactly once,
            // here.
            unsafe { drop(Box::from_raw(self.ptr.as_ptr())) };
        }
    }

    #[derive(Clone, Copy)]
    enum Mode {
        MarkHeader,
        AddItems(u32),
        RemoveFirstItem,
    }

    struct App {
        header: Slot<Header>,
        items: Vec<DynItem>,
        log: Log,
        mode: Cell<Mode>,
    }

    impl App {
        fn new(log: Log, mode: Mode) -> Self {
            App {
                header: Slot::new(Header {
                    value: 0,
                    log: log.clone(),
                }),
                items: Vec::new(),
                log,
                mode: Cell::new(mode),
            }
        }
    }

    impl Node<DirtyList> for App {
        fn mount(&mut self, mounter: &mut Mounter<'_, Self, DirtyList>) {
            // SAFETY: `header` is a field, and `unmount` deregisters it.
            unsafe { mounter.slot(&mut self.header) };
        }

        fn unmount(&mut self, unmounter: &mut Unmounter<'_, DirtyList>) {
            unmounter.slot(&mut self.header);
            for item in &mut self.items {
                // SAFETY: `item.ptr` is the live `Item` for `item.handle`, freed when `item` drops.
                unsafe { unmounter.child::<Item>(item.ptr, item.handle) };
            }
        }

        fn trigger(&mut self, cx: &mut TriggerCtx<'_, DirtyList>, op: &mut DirtyList) {
            self.log.order.borrow_mut().push(0);

            match self.mode.get() {
                // Mark a child through the operation, so the driver revisits it later in this drain.
                Mode::MarkHeader => op.mark(self.header.handle()),

                Mode::AddItems(n) => {
                    for i in 0..n {
                        let item = Box::new(Item {
                            id: 10 + i,
                            log: self.log.clone(),
                        });

                        // SAFETY: `Box::into_raw` is never null.
                        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(item)) };

                        // SAFETY: `ptr` is a fresh allocation owned by the `DynItem` pushed below and
                        // freed only when its handle is removed.
                        let handle = unsafe { cx.insert_child::<Item>(ptr) };

                        self.items.push(DynItem { ptr, handle });
                        op.mark(handle);
                    }
                }

                Mode::RemoveFirstItem => {
                    if !self.items.is_empty() {
                        let dyn_item = self.items.remove(0);

                        // Deregister the subtree before freeing, so a later trigger of the handle reports
                        // it gone rather than dereferencing freed memory.
                        // SAFETY: `dyn_item.ptr` is the live `Item` for `dyn_item.handle`; its box is freed
                        // only by the `drop` below, after this returns.
                        unsafe { cx.unmounter().child::<Item>(dyn_item.ptr, dyn_item.handle) };

                        drop(dyn_item);
                    }
                }
            }
        }
    }

    #[test]
    fn tree_survives_being_moved() {
        let log = Log::default();
        let mut dirty = DirtyList::default();
        let mut tree = Tree::new(App::new(log.clone(), Mode::AddItems(2)));

        let app = tree.root_handle();
        dirty.mark(app);
        dirty.flush(&mut tree);
        assert_eq!(*log.order.borrow(), vec![0, 10, 11]);

        let mut held = vec![tree];
        let tree = held.pop().unwrap();
        let mut tree = std::convert::identity(tree);

        log.order.borrow_mut().clear();
        let item = tree.root_node().items[1].handle;
        tree.trigger(item, &mut dirty);
        assert_eq!(*log.order.borrow(), vec![11]);
    }

    #[test]
    fn a_node_marks_a_child_through_the_op_and_the_driver_drains_it() {
        let log = Log::default();
        let mut dirty = DirtyList::default();
        let mut tree = Tree::new(App::new(log.clone(), Mode::MarkHeader));

        let app = tree.root_handle();
        dirty.mark(app);
        dirty.flush(&mut tree);

        // The app marked the header through the op; the driver drained it after the app because it
        // orders by depth.
        assert_eq!(*log.order.borrow(), vec![0, 1]);
        assert_eq!((*tree.root_node().header).value, 1);
    }

    #[test]
    fn dynamic_children_added_and_drained() {
        let log = Log::default();
        let mut dirty = DirtyList::default();
        let mut tree = Tree::new(App::new(log.clone(), Mode::AddItems(2)));

        let app = tree.root_handle();
        dirty.mark(app);
        dirty.flush(&mut tree);

        assert_eq!(*log.order.borrow(), vec![0, 10, 11]);
    }

    #[test]
    fn triggering_a_removed_handle_reports_it_gone() {
        let log = Log::default();
        let mut dirty = DirtyList::default();
        let mut tree = Tree::new(App::new(log.clone(), Mode::AddItems(2)));

        let app = tree.root_handle();
        dirty.mark(app);
        dirty.flush(&mut tree);
        let item0 = tree.root_node().items[0].handle;

        tree.root_node().mode.set(Mode::RemoveFirstItem);
        assert_eq!(log.freed.get(), 0);
        tree.trigger(app, &mut dirty);
        assert_eq!(log.freed.get(), 1);

        // The handle the driver still holds names a node that is gone, so the trigger does nothing and
        // says so rather than dereferencing freed memory.
        assert!(!tree.trigger(item0, &mut dirty));
    }

    #[test]
    fn removed_slot_is_reused_with_a_fresh_key() {
        let log = Log::default();
        let mut dirty = DirtyList::default();
        let mut tree = Tree::new(App::new(log.clone(), Mode::AddItems(1)));

        let app = tree.root_handle();
        dirty.mark(app);
        dirty.flush(&mut tree);
        let stale = tree.root_node().items[0].handle;

        tree.root_node().mode.set(Mode::RemoveFirstItem);
        tree.trigger(app, &mut dirty);

        tree.root_node().mode.set(Mode::AddItems(1));
        tree.trigger(app, &mut dirty);

        let live = tree.root_node().items[0].handle;
        assert_ne!(stale, live);
        assert!(
            !tree.trigger(stale, &mut dirty),
            "the old key does not resolve"
        );
        assert!(
            tree.trigger(live, &mut dirty),
            "the reused slot resolves under its fresh key"
        );
    }

    #[test]
    fn the_tree_owns_no_dirty_state_so_two_lists_drain_independently() {
        let log = Log::default();
        let mut tree = Tree::new(App::new(log.clone(), Mode::MarkHeader));
        let app = tree.root_handle();

        // Two unrelated drivers, each with its own list, both holding the same handle. The tree owns no
        // dirty state, so a node's marks land in whichever list is driving it, and draining one leaves
        // the other untouched.
        let mut build = DirtyList::default();
        let mut paint = DirtyList::default();
        build.mark(app);
        paint.mark(app);

        build.flush(&mut tree);
        assert_eq!(*log.order.borrow(), vec![0, 1]);
        assert!(paint.dirty.contains(&app), "the other list is undisturbed");

        log.order.borrow_mut().clear();
        paint.flush(&mut tree);
        assert_eq!(*log.order.borrow(), vec![0, 1]);
    }

    impl Operation for () {
        type Op<'a> = ();
    }

    // A three-level tree of inline `Slot` children, to exercise immediate recursive mount and a
    // read walk over the result.
    struct Leaf {
        id: u32,
    }

    impl Node<()> for Leaf {
        fn mount(&mut self, _mounter: &mut Mounter<'_, Self, ()>) {}

        fn unmount(&mut self, _unmounter: &mut Unmounter<'_, ()>) {}

        fn trigger(&mut self, _cx: &mut TriggerCtx<'_, ()>, (): ()) {}
    }

    struct Mid {
        id: u32,
        leaf: Slot<Leaf>,
    }
    impl Node<()> for Mid {
        fn mount(&mut self, mounter: &mut Mounter<'_, Self, ()>) {
            // SAFETY: `leaf` is a field, and `unmount` deregisters it.
            unsafe { mounter.slot(&mut self.leaf) };
        }

        fn unmount(&mut self, unmounter: &mut Unmounter<'_, ()>) {
            unmounter.slot(&mut self.leaf);
        }

        fn trigger(&mut self, _cx: &mut TriggerCtx<'_, ()>, (): ()) {}
    }

    struct Root {
        id: u32,
        mid: Slot<Mid>,
    }

    impl Node<()> for Root {
        fn mount(&mut self, mounter: &mut Mounter<'_, Self, ()>) {
            // SAFETY: `mid` is a field, and `unmount` deregisters it.
            unsafe { mounter.slot(&mut self.mid) };
        }

        fn unmount(&mut self, unmounter: &mut Unmounter<'_, ()>) {
            unmounter.slot(&mut self.mid);
        }

        fn trigger(&mut self, _cx: &mut TriggerCtx<'_, ()>, (): ()) {}
    }

    #[test]
    fn immediate_mount_registers_nested_children() {
        let tree: Tree<Root, ()> = Tree::new(Root {
            id: 0,
            mid: Slot::new(Mid {
                id: 1,
                leaf: Slot::new(Leaf { id: 2 }),
            }),
        });

        let root = tree.root_handle();
        let mid = tree.root_node().mid.handle();
        let leaf = tree.root_node().mid.leaf.handle();

        assert_eq!(tree.depth(root), Some(0));
        assert_eq!(tree.depth(mid), Some(1));
        assert_eq!(tree.depth(leaf), Some(2));
    }

    #[test]
    fn a_read_walk_forms_shared_refs_to_every_node() {
        let tree: Tree<Root, ()> = Tree::new(Root {
            id: 0,
            mid: Slot::new(Mid {
                id: 1,
                leaf: Slot::new(Leaf { id: 2 }),
            }),
        });

        // Descend reading shared references through the cells, the way a read-only walk would.
        let root: &Root = tree.root_node();
        let mid: &Mid = &root.mid;
        let leaf: &Leaf = &mid.leaf;

        assert_eq!(root.id, 0);
        assert_eq!(mid.id, 1);
        assert_eq!(leaf.id, 2);
    }

    #[test]
    fn root_is_freed_when_mount_panics() {
        struct Boom;
        impl Node<()> for Boom {
            fn mount(&mut self, _mounter: &mut Mounter<'_, Self, ()>) {
                panic!("mount panicked");
            }
            fn unmount(&mut self, _unmounter: &mut Unmounter<'_, ()>) {}
            fn trigger(&mut self, _cx: &mut TriggerCtx<'_, ()>, (): ()) {}
        }

        // The panic unwinds out of `new` before the `Tree` is built; miri's leak check confirms the root
        // box is freed anyway.
        let built = std::panic::catch_unwind(|| Tree::<Boom, ()>::new(Boom));
        assert!(built.is_err());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn forgetting_to_unmount_a_slot_panics_on_drop() {
        struct Bad {
            child: Slot<Leaf>,
        }
        impl Node<()> for Bad {
            fn mount(&mut self, mounter: &mut Mounter<'_, Self, ()>) {
                // SAFETY: `child` is a field of `Bad`. (`unmount` intentionally does not deregister it;
                // the tree is only dropped, never triggered after, so the lingering handle is never used.)
                unsafe { mounter.slot(&mut self.child) };
            }
            fn unmount(&mut self, _unmounter: &mut Unmounter<'_, ()>) {}
            fn trigger(&mut self, _cx: &mut TriggerCtx<'_, ()>, (): ()) {}
        }

        let tree = Tree::<Bad, ()>::new(Bad {
            child: Slot::new(Leaf { id: 0 }),
        });
        let dropped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(tree)));
        assert!(
            dropped.is_err(),
            "dropping a node that forgot to unmount its slot must panic in debug"
        );
    }
}

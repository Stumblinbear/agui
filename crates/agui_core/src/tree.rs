//! A statically-typed tree of reactive nodes, addressed by copyable [`NodeHandle`]s.
//!
//! The tree stores a root node and its descendants, each addressable by handle. It is a pure mechanism: it
//! holds each node's pointer, depth, and an opaque dispatch fn the consumer supplies, and reaches a node by
//! handle to run that fn. It never calls into node code itself; mounting, unmounting, and the recursion
//! through children are driven by the consumer through the raw [`Cursor`]. Looking a node up by handle is
//! O(1); [`Tree::depth`] orders a batch so parents act before children.
//!
//! A node owns its children's bodies: a [`Slot`] child inline in the node's own allocation, a [`BoxedSlot`]
//! child in a separate heap allocation. The registry holds only the handle, pointer, depth, and dispatch
//! fn; it never owns a body. A handle stays safe after its node is gone: [`Tree::dispatch`] returns `false`
//! instead of touching freed memory.

use std::mem::offset_of;
use std::ptr::NonNull;

use slotmap::{SlotMap, new_key_type};

new_key_type! {
    /// A small, copyable handle to a node. Good until that node is removed.
    pub struct NodeHandle;
}

/// The consumer's binding of how the tree dispatches to a node: the operation type [`Tree::dispatch`]
/// delivers. The operation may borrow for the duration of the call, such as a build op carrying a [`Cursor`].
pub trait NodeDispatch: Sized {
    /// The operation delivered to a node.
    type Operation<'a>;
}

/// The dispatch fn the consumer supplies for a node when registering it: given the node's pointer and an
/// operation, it casts the pointer to the node's concrete type and runs the node's handler. The tree
/// reaches node code only through this fn.
pub type DispatchGlue<N> = for<'a> unsafe fn(NonNull<()>, <N as NodeDispatch>::Operation<'a>);

struct Entry<N: NodeDispatch> {
    node: NonNull<()>,
    run: DispatchGlue<N>,
    depth: u32,
}

struct Registry<N: NodeDispatch> {
    heads: SlotMap<NodeHandle, Entry<N>>,
}

/// An inline child node bundled with its [`NodeHandle`]. Hold one in a field; register it with
/// [`Cursor::register`] while mounting and deregister it with [`Cursor::deregister`] while unmounting. Read
/// the child via [`get`](Self::get).
pub struct Slot<C> {
    value: C,
    handle: NodeHandle,
}

impl<C> Slot<C> {
    pub fn new(value: C) -> Self {
        Slot {
            handle: NodeHandle::default(),
            value,
        }
    }

    pub fn handle(&self) -> NodeHandle {
        self.handle
    }

    pub fn get(&self) -> &C {
        &self.value
    }

    pub fn get_mut(&mut self) -> &mut C {
        &mut self.value
    }
}

/// A heap-owned dynamic child, created with [`new`](Self::new), registered with [`Cursor::register`], and
/// torn down with [`Cursor::deregister`]. Frees its allocation on drop.
pub struct BoxedSlot<C> {
    ptr: NonNull<C>,
    handle: NodeHandle,
}

impl<C> BoxedSlot<C> {
    /// Boxes `child` as an unregistered dynamic child. Register it below a node with
    /// [`Cursor::register`], then mount it.
    pub fn new(child: C) -> Self {
        BoxedSlot {
            handle: NodeHandle::default(),
            ptr: NonNull::from(Box::leak(Box::new(child))),
        }
    }

    pub fn handle(&self) -> NodeHandle {
        self.handle
    }

    pub fn get(&self) -> &C {
        // SAFETY: live until this `BoxedSlot` drops.
        unsafe { &*self.ptr.as_ptr() }
    }

    pub fn get_mut(&mut self) -> &mut C {
        // SAFETY: as `get`, and `&mut self` rules out another borrow.
        unsafe { &mut *self.ptr.as_ptr() }
    }
}

impl<C> Drop for BoxedSlot<C> {
    fn drop(&mut self) {
        // SAFETY: from `Box::leak` in `BoxedSlot::new`, freed once.
        unsafe { drop(Box::from_raw(self.ptr.as_ptr())) };
    }
}

/// Storage for a child node a [`Cursor`] can register and later re-enter: an inline [`Slot`], a heap
/// [`BoxedSlot`], or a consumer's own shape. It yields the child's address for registration, the typed child
/// for reconcile, and the handle it is registered under, so one [`register`](Cursor::register) and one
/// [`with_child`](Cursor::with_child) serve every container.
///
/// # Safety
/// [`body`](Self::body) must return the true address of the contained child node, [`node_mut`](Self::node_mut)
/// must return that same child, and [`handle`](Self::handle) must return the handle it was last registered
/// under. [`with_child`](Cursor::with_child) trusts all three to line up.
pub unsafe trait NodeContainer {
    /// The contained child's type.
    type Node;

    /// The contained child's address. `parent` is the base pointer of the node owning this storage: an
    /// inline `Slot` offsets from it; a heap `BoxedSlot` ignores it.
    ///
    /// # Safety
    /// `parent` must be the base of the node that owns `self`.
    unsafe fn body(&self, parent: NonNull<()>) -> NonNull<()>;

    /// The handle this container is registered under, or the default handle before registration.
    fn handle(&self) -> NodeHandle;

    /// Mutable access to the handle this container records: [`register`](Cursor::register) sets it,
    /// [`deregister`](Cursor::deregister) takes it back.
    fn handle_mut(&mut self) -> &mut NodeHandle;

    /// The contained child, for reconcile through [`with_child`](Cursor::with_child).
    fn node_mut(&mut self) -> &mut Self::Node;
}

// SAFETY: `body` and `node_mut` return `value`, the inline child this slot holds, and `handle` returns the
// recorded handle.
unsafe impl<C> NodeContainer for Slot<C> {
    type Node = C;

    unsafe fn body(&self, parent: NonNull<()>) -> NonNull<()> {
        let base = parent.as_ptr() as usize;
        let off =
            (std::ptr::from_ref(self) as usize).wrapping_sub(base) + offset_of!(Slot<C>, value);
        // The address is raw arithmetic from `parent`, keeping that provenance rather than a reborrow of
        // `self`; the registered pointer is dereferenced later, after such a reborrow would be dead.
        // SAFETY: `off` lands within `parent`'s allocation (the caller's contract that `self` is its
        // field), and the result is non-null since it derives from the non-null `parent`.
        unsafe { NonNull::new_unchecked(parent.as_ptr().cast::<u8>().add(off).cast::<()>()) }
    }

    fn handle(&self) -> NodeHandle {
        self.handle
    }

    fn handle_mut(&mut self) -> &mut NodeHandle {
        &mut self.handle
    }

    fn node_mut(&mut self) -> &mut C {
        &mut self.value
    }
}

// SAFETY: `body` and `node_mut` return the boxed child at `ptr`, and `handle` returns the recorded handle.
unsafe impl<C> NodeContainer for BoxedSlot<C> {
    type Node = C;

    unsafe fn body(&self, _parent: NonNull<()>) -> NonNull<()> {
        self.ptr.cast::<()>()
    }

    fn handle(&self) -> NodeHandle {
        self.handle
    }

    fn handle_mut(&mut self) -> &mut NodeHandle {
        &mut self.handle
    }

    fn node_mut(&mut self) -> &mut C {
        self.get_mut()
    }
}

/// The raw tree-editing primitive a node reaches during a lifecycle hook, positioned at that node.
/// Registers and deregisters children (storing the dispatch glue the consumer supplies) and hands back
/// each child's own cursor; the consumer's contexts wrap it to add their data and drive the recursion.
pub struct Cursor<'a, N: NodeDispatch> {
    reg: &'a mut Registry<N>,
    this: NonNull<()>,
    handle: NodeHandle,
    depth: u32,
}

impl<N: NodeDispatch> Cursor<'_, N> {
    /// The handle of the node this cursor is positioned at.
    pub fn handle(&self) -> NodeHandle {
        self.handle
    }

    /// The address of the node this cursor is positioned at, carrying provenance over its allocation. A
    /// consumer mints a long-lived pointer into the node from this (rather than from a `&mut` to the node),
    /// so it survives the node's later reborrows.
    pub fn this(&self) -> NonNull<()> {
        self.this
    }

    /// Re-bases this cursor onto `this`, the real address of the node it is positioned at, so the node's
    /// inline children register against `this`. Call it for a node reached through a heap indirection (a
    /// `Box<dyn …>`), whose registered pointer is the box rather than the node behind it.
    ///
    /// # Safety
    /// `this` must be this cursor's node's address, carrying provenance over that node's whole allocation:
    /// pass `addr_of_mut!(**boxed)`, not `&mut **boxed`, whose reborrow tag would die before a child is
    /// dereferenced.
    pub unsafe fn rebase(&mut self, this: NonNull<()>) {
        self.this = this;
    }

    /// Registers `child` (an inline [`Slot`] or a heap [`BoxedSlot`]) below this node under `glue`, and
    /// returns its cursor. The caller mounts the child through its `get_mut`.
    ///
    /// # Safety
    /// - `child` must belong to the node at `self.this`.
    /// - `glue` must dispatch the child's node type.
    /// - The holder must deregister `child` with [`deregister`](Self::deregister) before it drops.
    pub unsafe fn register<S: NodeContainer>(
        &mut self,
        child: &mut S,
        glue: DispatchGlue<N>,
    ) -> Cursor<'_, N> {
        // SAFETY: `self.this` is this node's base, which `body` needs; the rest is the caller's contract.
        let node = unsafe { child.body(self.this) };
        let depth = self.depth + 1;
        let handle = self.reg.heads.insert(Entry {
            node,
            run: glue,
            depth,
        });
        *child.handle_mut() = handle;
        Cursor {
            reg: &mut *self.reg,
            this: node,
            handle,
            depth,
        }
    }

    /// Deregisters `child` (an inline [`Slot`] or a heap [`BoxedSlot`]). Call after recursing into its
    /// teardown.
    pub fn deregister<S: NodeContainer>(&mut self, child: &mut S) {
        let handle = std::mem::take(child.handle_mut());
        self.reg.heads.remove(handle);
    }

    /// Reconciles an already-registered child in place: hands `f` the child and a cursor positioned at it.
    /// Works for any [`NodeContainer`]: an inline [`Slot`], a heap [`BoxedSlot`], or a consumer's own. Unlike
    /// [`register`](Self::register) it adds nothing; the child keeps the handle it was given at registration.
    ///
    /// # Safety
    /// `child` must belong to the node this cursor is positioned at.
    pub unsafe fn with_child<S: NodeContainer, R>(
        &mut self,
        child: &mut S,
        f: impl FnOnce(&mut S::Node, Cursor<'_, N>) -> R,
    ) -> R {
        let handle = child.handle();
        // SAFETY: the caller guarantees `child` belongs to `self.this`, which is `body`'s precondition. The
        // recomputed pointer is the one registered for `handle`.
        let this = unsafe { child.body(self.this) };
        let depth = self.depth + 1;
        let cursor = Cursor {
            reg: &mut *self.reg,
            this,
            handle,
            depth,
        };
        f(child.node_mut(), cursor)
    }
}

/// A tree of reactive nodes: the root and its descendants, each addressable by handle. See the module docs.
pub struct Tree<R, N: NodeDispatch> {
    reg: Registry<N>,
    root: NonNull<R>,
    root_handle: NodeHandle,
}

impl<R, N: NodeDispatch> Tree<R, N> {
    /// Registers `root` as the tree's root, dispatched by `glue`, and mounts it: `mount` is handed the root
    /// and its cursor so the consumer registers the root's children. Building a tree mounts it.
    pub fn new(root: R, glue: DispatchGlue<N>, mount: impl FnOnce(&mut R, Cursor<'_, N>)) -> Self {
        let mut reg = Registry {
            heads: SlotMap::with_key(),
        };
        // SAFETY: `Box::into_raw` is never null.
        let root = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(root))) };
        let root_handle = reg.heads.insert(Entry {
            node: root.cast::<()>(),
            run: glue,
            depth: 0,
        });
        let mut tree = Tree {
            reg,
            root,
            root_handle,
        };

        // SAFETY: the live root, whose body is disjoint from the registry the cursor holds.
        let root_ref = unsafe { &mut *tree.root.as_ptr() };
        let cursor = Cursor {
            reg: &mut tree.reg,
            this: tree.root.cast::<()>(),
            handle: tree.root_handle,
            depth: 0,
        };
        mount(root_ref, cursor);

        tree
    }

    pub fn root_handle(&self) -> NodeHandle {
        self.root_handle
    }
    pub fn root(&self) -> &R {
        // SAFETY: live root; `&self` rules out a `&mut`.
        unsafe { &*self.root.as_ptr() }
    }
    pub fn depth(&self, h: NodeHandle) -> Option<u32> {
        self.reg.heads.get(h).map(|e| e.depth)
    }

    /// Dispatches a cursor-bearing operation to the node `h` names: vends its cursor to `make`, which wraps
    /// it into the operation, then runs its dispatch fn. Returns `false` if the node is gone.
    pub fn dispatch_with_cursor<'a>(
        &'a mut self,
        h: NodeHandle,
        make: impl FnOnce(Cursor<'a, N>) -> N::Operation<'a>,
    ) -> bool {
        let Some(entry) = self.reg.heads.get(h) else {
            return false;
        };
        let (node, run, depth) = (entry.node, entry.run, entry.depth);
        let cursor = Cursor {
            reg: &mut self.reg,
            this: node,
            handle: h,
            depth,
        };
        let operation = make(cursor);
        // SAFETY: `node` is the live body for `h`, disjoint from the registry the cursor holds.
        unsafe { run(node, operation) };
        true
    }

    /// Vends a cursor at the node `h` names to `f`, returning its result, or `None` if the node is gone. `f`
    /// registers and reconciles children through the cursor. Unlike [`dispatch_with_cursor`](Self::dispatch_with_cursor)
    /// it runs no dispatch fn and so does not reborrow the node as `&mut`; a caller already holding a borrow
    /// into the node may reconcile its children through this.
    pub fn with_cursor<'a, Ret>(
        &'a mut self,
        h: NodeHandle,
        f: impl FnOnce(Cursor<'a, N>) -> Ret,
    ) -> Option<Ret> {
        let entry = self.reg.heads.get(h)?;
        let (node, depth) = (entry.node, entry.depth);
        let cursor = Cursor {
            reg: &mut self.reg,
            this: node,
            handle: h,
            depth,
        };
        Some(f(cursor))
    }

    /// Dispatches a cursorless operation (no structural changes) to the node `h` names. Returns `false` if
    /// the node is gone.
    pub fn dispatch(&mut self, h: NodeHandle, operation: N::Operation<'_>) -> bool {
        let Some(entry) = self.reg.heads.get(h) else {
            return false;
        };
        let (node, run) = (entry.node, entry.run);
        // SAFETY: `node` is the live body for `h`.
        unsafe { run(node, operation) };
        true
    }

    /// Runs `f` on the node `h` names, viewed as a `C`, returning its result. `None` if the node is gone.
    /// Unlike [`dispatch`](Self::dispatch) this needs no glue, since the caller names the type.
    ///
    /// # Safety
    /// The node at `h` must have type `C`.
    pub unsafe fn with_node<C, T>(
        &mut self,
        h: NodeHandle,
        f: impl FnOnce(&mut C) -> T,
    ) -> Option<T> {
        let node = self.reg.heads.get(h)?.node;
        // SAFETY: the caller guarantees the node at `h` is a `C`, and `&mut self` rules out aliasing.
        let node = unsafe { &mut *node.cast::<C>().as_ptr() };
        Some(f(node))
    }
}

impl<R, N: NodeDispatch> Drop for Tree<R, N> {
    fn drop(&mut self) {
        // Freeing the root recursively drops every owned child (inline `Slot`s and `BoxedSlot`s); the
        // registry holds only borrowed pointers, so dropping it frees nothing.
        // SAFETY: `root` came from `Box::into_raw` in `new`, freed once here.
        unsafe { drop(Box::from_raw(self.root.as_ptr())) };
    }
}

#[cfg(test)]
mod tests {
    use std::ptr::NonNull;

    use super::{BoxedSlot, Cursor, NodeDispatch, NodeHandle, Slot, Tree};

    // The consumer side. The tree imposes no node trait, so the consumer defines its own element trait and
    // the dispatch glue the tree stores. Mount and unmount are concrete lifecycle methods called directly;
    // `Operation` is only what the tree dispatches by handle, and the glue unwraps it onto a per-op method.
    struct App;

    impl NodeDispatch for App {
        type Operation<'a> = Operation<'a>;
    }

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
        fn message(&mut self, message: u32) {
            let _ = message;
        }
    }

    unsafe fn run<C: Element>(data: NonNull<()>, operation: Operation<'_>) {
        // SAFETY: only ever stored against a `data` that points at a live `C`.
        let node = unsafe { &mut *data.cast::<C>().as_ptr() };
        match operation {
            Operation::Build(ctx) => node.build(ctx),
            Operation::Message(message) => node.message(message),
        }
    }

    #[derive(Clone, Copy)]
    struct Scope(u32);

    struct MountCtx<'a> {
        cursor: Cursor<'a, App>,
        scope: Scope,
    }
    struct UnmountCtx<'a> {
        cursor: Cursor<'a, App>,
    }
    struct BuildCtx<'a> {
        cursor: Cursor<'a, App>,
        target: usize,
    }
    // `Build` carries a cursor (may restructure); `Message` carries none.
    enum Operation<'a> {
        Build(BuildCtx<'a>),
        Message(u32),
    }

    impl MountCtx<'_> {
        fn child<C: Element>(&mut self, slot: &mut Slot<C>) {
            let scope = self.scope;
            // SAFETY: `slot` is the caller's field, `run::<C>` dispatches it, and its teardown deregisters
            // it.
            let cursor = unsafe { self.cursor.register(slot, run::<C>) };
            slot.get_mut().mount(MountCtx { cursor, scope });
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
            owner.get_mut().mount(MountCtx {
                cursor,
                scope: Scope(0),
            });
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

    struct Leaf {
        scope_seen: u32,
        messages: u32,
    }
    impl Element for Leaf {
        fn mount(&mut self, ctx: MountCtx<'_>) {
            self.scope_seen = ctx.scope.0;
        }
        fn message(&mut self, message: u32) {
            self.messages += message;
        }
    }

    fn leaf() -> Leaf {
        Leaf {
            scope_seen: 0,
            messages: 0,
        }
    }

    struct Root {
        a: Slot<Leaf>,
        b: Slot<Leaf>,
    }
    impl Element for Root {
        fn mount(&mut self, mut ctx: MountCtx<'_>) {
            ctx.child(&mut self.a);
            ctx.child(&mut self.b);
        }
        fn unmount(&mut self, mut ctx: UnmountCtx<'_>) {
            ctx.child(&mut self.a);
            ctx.child(&mut self.b);
        }
    }

    struct Dynamic {
        kids: Vec<BoxedSlot<Leaf>>,
    }
    impl Element for Dynamic {
        fn unmount(&mut self, mut ctx: UnmountCtx<'_>) {
            for kid in &mut self.kids {
                ctx.boxed(kid);
            }
        }
        fn build(&mut self, mut ctx: BuildCtx<'_>) {
            let target = ctx.target;
            while self.kids.len() < target {
                self.kids.push(ctx.insert(leaf()));
            }
            while self.kids.len() > target {
                let kid = self.kids.pop().unwrap();
                ctx.remove(kid);
            }
        }
    }

    fn mounted_root() -> Tree<Root, App> {
        Tree::<Root, App>::new(
            Root {
                a: Slot::new(leaf()),
                b: Slot::new(leaf()),
            },
            run::<Root>,
            |root, cursor| {
                root.mount(MountCtx {
                    cursor,
                    scope: Scope(7),
                });
            },
        )
    }

    #[test]
    fn mount_registers_inline_children_and_threads_the_scope() {
        let tree = mounted_root();
        assert_eq!(tree.depth(tree.root_handle()), Some(0));
        assert_eq!(tree.depth(tree.root().a.handle()), Some(1));
        assert_eq!(tree.depth(tree.root().b.handle()), Some(1));
        assert_eq!(
            tree.root().a.get().scope_seen,
            7,
            "scope threaded to the child"
        );
    }

    #[test]
    fn message_reaches_an_inline_child_by_handle() {
        let mut tree = mounted_root();
        let a = tree.root().a.handle();
        assert!(tree.dispatch(a, Operation::Message(5)));
        assert_eq!(tree.root().a.get().messages, 5);
        assert!(!tree.dispatch(NodeHandle::default(), Operation::Message(1)));
    }

    #[test]
    fn with_node_views_a_node_by_handle_when_the_type_is_known() {
        let mut tree = mounted_root();
        let a = tree.root().a.handle();
        tree.dispatch(a, Operation::Message(9));

        // SAFETY: `a` names a `Leaf`.
        let messages = unsafe { tree.with_node::<Leaf, _>(a, |leaf| leaf.messages) };
        assert_eq!(messages, Some(9));
        // SAFETY: the type is irrelevant for a dead handle; `None` comes back first.
        let gone =
            unsafe { tree.with_node::<Leaf, _>(NodeHandle::default(), |leaf| leaf.messages) };
        assert_eq!(gone, None);
    }

    #[test]
    fn build_grows_and_shrinks_a_dynamic_subtree() {
        let mut tree = Tree::<Dynamic, App>::new(
            Dynamic { kids: Vec::new() },
            run::<Dynamic>,
            |root, cursor| {
                root.mount(MountCtx {
                    cursor,
                    scope: Scope(0),
                });
            },
        );
        let root = tree.root_handle();

        tree.dispatch_with_cursor(root, |cursor| {
            Operation::Build(BuildCtx { cursor, target: 3 })
        });
        assert_eq!(tree.root().kids.len(), 3);
        let kept = tree.root().kids[0].handle();
        let removed = tree.root().kids[2].handle();
        assert_eq!(tree.depth(kept), Some(1));
        assert_eq!(tree.depth(removed), Some(1));

        tree.dispatch_with_cursor(root, |cursor| {
            Operation::Build(BuildCtx { cursor, target: 1 })
        });
        assert_eq!(tree.root().kids.len(), 1);
        assert!(
            tree.dispatch(kept, Operation::Message(0)),
            "the surviving kid still resolves"
        );
        assert!(
            !tree.dispatch(removed, Operation::Message(0)),
            "a removed kid's handle is dead"
        );
    }

    // Holds both an inline and a dynamic child. Its first build inserts the dynamic child; its second
    // re-enters both children and records the handles their cursors carry.
    struct ReEnter {
        inline: Slot<Leaf>,
        kids: Vec<BoxedSlot<Leaf>>,
        entered_inline: Option<NodeHandle>,
        entered_kid: Option<NodeHandle>,
    }
    impl Element for ReEnter {
        fn mount(&mut self, mut ctx: MountCtx<'_>) {
            ctx.child(&mut self.inline);
        }
        fn unmount(&mut self, mut ctx: UnmountCtx<'_>) {
            ctx.child(&mut self.inline);
            for kid in &mut self.kids {
                ctx.boxed(kid);
            }
        }
        fn build(&mut self, mut ctx: BuildCtx<'_>) {
            if self.kids.is_empty() {
                self.kids.push(ctx.insert(leaf()));
            } else {
                // SAFETY: `inline` and `kids[0]` are this element's own children.
                unsafe {
                    ctx.cursor.with_child(&mut self.inline, |_child, cursor| {
                        self.entered_inline = Some(cursor.handle());
                    });

                    ctx.cursor.with_child(&mut self.kids[0], |_child, cursor| {
                        self.entered_kid = Some(cursor.handle());
                    });
                }
            }
        }
    }

    #[test]
    fn enter_returns_the_existing_child_cursors() {
        let mut tree = Tree::<ReEnter, App>::new(
            ReEnter {
                inline: Slot::new(leaf()),
                kids: Vec::new(),
                entered_inline: None,
                entered_kid: None,
            },
            run::<ReEnter>,
            |root, cursor| {
                root.mount(MountCtx {
                    cursor,
                    scope: Scope(0),
                });
            },
        );
        let root = tree.root_handle();

        tree.dispatch_with_cursor(root, |cursor| {
            Operation::Build(BuildCtx { cursor, target: 0 })
        });
        tree.dispatch_with_cursor(root, |cursor| {
            Operation::Build(BuildCtx { cursor, target: 0 })
        });

        let inline = tree.root().inline.handle();
        let kid = tree.root().kids[0].handle();
        assert_eq!(
            tree.root().entered_inline,
            Some(inline),
            "re-entered the inline child"
        );
        assert_eq!(
            tree.root().entered_kid,
            Some(kid),
            "re-entered the dynamic child"
        );
        assert_eq!(tree.depth(inline), Some(1));
    }

    // A two-level inline subtree. Reconcile re-enters `mid` off the root's base, then re-enters `mid`'s own
    // `leaf` off that recomputed pointer, so the inline derivation chains under a live `&mut` at each level:
    // the path `with_child` takes for an inline `Slot`.
    struct DeepMid {
        leaf: Slot<Leaf>,
    }
    impl Element for DeepMid {
        fn mount(&mut self, mut ctx: MountCtx<'_>) {
            ctx.child(&mut self.leaf);
        }
    }

    struct DeepRoot {
        mid: Slot<DeepMid>,
    }
    impl Element for DeepRoot {
        fn mount(&mut self, mut ctx: MountCtx<'_>) {
            ctx.child(&mut self.mid);
        }

        fn build(&mut self, mut ctx: BuildCtx<'_>) {
            // SAFETY: `mid` is this element's own child, and `mid.leaf` is `mid`'s.
            unsafe {
                ctx.cursor.with_child(&mut self.mid, |mid, mut cursor| {
                    cursor.with_child(&mut mid.leaf, |leaf, _cursor| leaf.messages += 1);
                });
            }
        }
    }

    #[test]
    fn reconcile_reenters_a_deep_inline_subtree() {
        let mut tree = Tree::<DeepRoot, App>::new(
            DeepRoot {
                mid: Slot::new(DeepMid {
                    leaf: Slot::new(leaf()),
                }),
            },
            run::<DeepRoot>,
            |root, cursor| {
                root.mount(MountCtx {
                    cursor,
                    scope: Scope(0),
                });
            },
        );
        let root = tree.root_handle();

        tree.dispatch_with_cursor(root, |cursor| {
            Operation::Build(BuildCtx { cursor, target: 0 })
        });

        let leaf = tree.root().mid.get().leaf.handle();
        assert_eq!(
            tree.depth(leaf),
            Some(2),
            "the inline grandchild keeps its depth"
        );
        assert_eq!(
            tree.root().mid.get().leaf.get().messages,
            1,
            "reconcile reached the inline grandchild"
        );
    }
}

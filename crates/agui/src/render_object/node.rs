use std::cell::{Cell, UnsafeCell};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

use typed_floats::{Positive, PositiveFinite};

use crate::{
    context::PaintCtx,
    diagnostics::{Diagnostics, DiagnosticsNode},
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    pipeline::render_pipeline::{LayoutBoundary, LayoutBoundaryHandle},
    prelude::render_object::LayoutScope,
    render_object::{
        LayoutCtx, RenderObject,
        box_layout::{BoxConstraints, RenderBox},
        sliver::RenderSliver,
    },
    semantics::SemanticsTreeBuilder,
    text::TextBaseline,
};

/// Storage for an element's render object. The render object is interior-mutable, so a parent reaching it
/// during layout and an isolated relayout can both borrow it as `&mut` across the `&mut element` retags of
/// rebuilds without aliasing UB. Borrowed only during a layout or paint pass.
///
/// An element holds its render object in one of these and hands a parent a pointer to it with
/// [`render_object_ptr`](Self::render_object_ptr). Render-object code never sees the cell: it receives a plain
/// `&mut R`.
///
/// In debug it asserts no overlapping borrow; in release it is a bare [`UnsafeCell`], the same size as `R`.
pub struct RenderObjectCell<R: ?Sized> {
    inner: UnsafeCell<RenderObjectInner<R>>,
}

/// The interior of a [`RenderObjectCell`]: the render object plus, in debug, a borrow flag. The flag lives
/// inside the cell so it is reachable through the same `.get()`-derived pointer as the render object.
struct RenderObjectInner<R: ?Sized> {
    #[cfg(debug_assertions)]
    borrows: Cell<isize>,
    // Last field, so `RenderObjectInner<Concrete>` unsizes to `RenderObjectInner<dyn …>`.
    value: R,
}

impl<R> RenderObjectCell<R> {
    pub fn new(value: R) -> Self {
        Self {
            inner: UnsafeCell::new(RenderObjectInner {
                #[cfg(debug_assertions)]
                borrows: Cell::new(0),
                value,
            }),
        }
    }
}

impl<R: ?Sized> RenderObjectCell<R> {
    /// The render object, by shared reference. For reading outside a layout or paint pass, when no
    /// [`render_object_ptr`](Self::render_object_ptr) borrow is live.
    pub fn get(&self) -> &R {
        // SAFETY: a shared read taken outside a pass, so it does not alias the pass-time pointer borrow.
        unsafe { &(*self.inner.get()).value }
    }

    /// The render object, by exclusive reference.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner.get_mut().value
    }

    /// A pointer to the render object, taken via `.get()` so it carries interior-mutable provenance. A
    /// parent's [`RenderNode`] resolves one of these each pass and borrows it.
    pub fn render_object_ptr(&self) -> RenderObjectPtr<R> {
        // SAFETY: `.get()` is the blessed interior-mutable derivation; the pointer is borrowed only at pass time.
        RenderObjectPtr(unsafe { NonNull::new_unchecked(self.inner.get()) })
    }
}

/// A pointer to a render object's [`RenderObjectCell`], resolved during a pass and borrowed to reach the
/// render object. An element produces one with [`Element::render_object_ptr`]; a parent's [`RenderNode`]
/// resolves it fresh each pass from the child's address.
///
/// [`Element::render_object_ptr`]: crate::element::Element::render_object_ptr
pub struct RenderObjectPtr<R: ?Sized>(NonNull<RenderObjectInner<R>>);

impl<R> RenderObjectPtr<R> {
    /// A placeholder for a render-less element (`Render = ()`), never dereferenced.
    pub fn dangling() -> Self {
        RenderObjectPtr(NonNull::dangling())
    }
}

impl<R: RenderBox> RenderObjectPtr<R> {
    /// Erases this to the boxed protocol, for a wrapper that adapts a concrete child into `dyn RenderBox`.
    pub fn into_box(self) -> RenderObjectPtr<dyn RenderBox> {
        // The annotation drives the unsizing coercion; `as` could not.
        let inner: *mut RenderObjectInner<dyn RenderBox> = self.0.as_ptr();
        // SAFETY: non-null, since it came from a `NonNull`.
        RenderObjectPtr(unsafe { NonNull::new_unchecked(inner) })
    }
}

impl<R: RenderSliver> RenderObjectPtr<R> {
    /// Erases this to the sliver protocol, for a wrapper that adapts a concrete child into `dyn RenderSliver`.
    pub fn into_sliver(self) -> RenderObjectPtr<dyn RenderSliver> {
        let inner: *mut RenderObjectInner<dyn RenderSliver> = self.0.as_ptr();
        // SAFETY: non-null, since it came from a `NonNull`.
        RenderObjectPtr(unsafe { NonNull::new_unchecked(inner) })
    }
}

/// A scoped shared borrow of a render object reached through a [`RenderNode`].
pub struct RenderObjectRef<'a, R: ?Sized> {
    inner: NonNull<RenderObjectInner<R>>,
    _marker: PhantomData<&'a R>,
}

impl<R: ?Sized> Deref for RenderObjectRef<'_, R> {
    type Target = R;

    fn deref(&self) -> &R {
        // SAFETY: `RenderNode::borrow` established access for this guard's lifetime.
        unsafe { &(*self.inner.as_ptr()).value }
    }
}

/// A scoped exclusive borrow of a render object reached through a [`RenderNode`].
pub struct RenderObjectMut<'a, R: ?Sized> {
    inner: NonNull<RenderObjectInner<R>>,
    _marker: PhantomData<&'a mut R>,
}

impl<R: ?Sized> Deref for RenderObjectMut<'_, R> {
    type Target = R;

    fn deref(&self) -> &R {
        // SAFETY: `RenderNode::borrow_mut` established access for this guard's lifetime.
        unsafe { &(*self.inner.as_ptr()).value }
    }
}

impl<R: ?Sized> DerefMut for RenderObjectMut<'_, R> {
    fn deref_mut(&mut self) -> &mut R {
        // SAFETY: as `deref`.
        unsafe { &mut (*self.inner.as_ptr()).value }
    }
}

#[cfg(debug_assertions)]
impl<R: ?Sized> Drop for RenderObjectRef<'_, R> {
    fn drop(&mut self) {
        // SAFETY: the cell outlives the borrow held during the pass.
        let borrows = unsafe { &(*self.inner.as_ptr()).borrows };
        borrows.set(borrows.get() - 1);
    }
}

#[cfg(debug_assertions)]
impl<R: ?Sized> Drop for RenderObjectMut<'_, R> {
    fn drop(&mut self) {
        // SAFETY: the cell outlives the borrow held during the pass.
        unsafe { (*self.inner.as_ptr()).borrows.set(0) };
    }
}

/// A parent's deferred handle to a child render object: the child node's address plus how to resolve its
/// render object pointer from it. Resolved fresh each pass — never dereferenced at mount, where the protected
/// `&mut element` mount chain would forbid reaching the child through this separate address.
pub struct MountedChild<R: ?Sized> {
    node: NonNull<()>,
    resolve: unsafe fn(NonNull<()>) -> RenderObjectPtr<R>,
}

impl<R: ?Sized> MountedChild<R> {
    /// Builds the handle from the child node's `address` and a `resolve` that projects it to the child's
    /// render object pointer. [`UpdateCtx::mount`](crate::context::UpdateCtx::mount) is the only caller.
    ///
    /// # Safety
    /// `address` must be the mounted child node's address, valid for as long as the child is mounted, and
    /// `resolve` must project it to that child's render object pointer.
    pub unsafe fn new(
        address: NonNull<()>,
        resolve: unsafe fn(NonNull<()>) -> RenderObjectPtr<R>,
    ) -> Self {
        Self {
            node: address,
            resolve,
        }
    }

    /// Resolves and exclusively borrows the child render object, for a boundary holding this handle to re-lay or
    /// repaint it. As with [`RenderNode::borrow_mut`], call only during a pass.
    pub fn borrow_mut(&self) -> RenderObjectMut<'_, R> {
        // SAFETY: as `RenderNode::resolve` — pass time, no element hook on the stack.
        let inner = unsafe { (self.resolve)(self.node) }.0;
        debug_mark_exclusive(inner);
        RenderObjectMut {
            inner,
            _marker: PhantomData,
        }
    }
}

// The node address and resolver are plain data; copying the handle just copies them.
impl<R: ?Sized> Clone for MountedChild<R> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<R: ?Sized> Copy for MountedChild<R> {}

/// A relayout boundary for an inline box render object: the deferred handle to it plus the box constraints it
/// last took. At flush it resolves the handle and re-lays the render object under those constraints.
struct InlineBoxBoundary<R: RenderBox + ?Sized> {
    handle: MountedChild<R>,
    constraints: BoxConstraints,
}

impl<R: RenderBox + ?Sized> LayoutBoundary for InlineBoxBoundary<R> {
    fn relayout(&mut self, ctx: &mut LayoutCtx) {
        self.handle.borrow_mut().layout(ctx, self.constraints);
    }
}

/// A parent render object's pointer to one child render object, paired with the per-child layout state the
/// parent keeps about it (its [`parent_data`](Self::parent_data) and pipeline flags).
///
/// The child render object is owned by the child's element, not here; this holds only the deferred handle to
/// it ([`MountedChild`]), [`set`](Self::set) once the child is attached. The handle is resolved fresh each
/// pass and the resulting render object is interior-mutable, so it survives the child's rebuilds and an
/// isolated relayout can reach it alongside a normal layout. Resolving is the one `unsafe` step, kept off the
/// render objects that hold the node: they call the safe forwarding methods below, which borrow the child for
/// one call.
pub struct RenderNode<R: ?Sized, P = ()> {
    pub parent_data: P,

    needs_compositing: bool,

    /// Registered only while the child is its own relayout boundary; dropping it unregisters. The box layout
    /// forwarders set and clear it, and `set`/`clear` drop it so it never outlives the child it holds.
    boundary: Option<LayoutBoundaryHandle>,

    /// Deferred handle to the child render object, `None` until [`set`](Self::set) wires it at mount.
    child: Option<MountedChild<R>>,
}

impl<R: ?Sized, P: Default> RenderNode<R, P> {
    pub fn new(parent_data: P) -> Self {
        Self {
            parent_data,

            needs_compositing: false,

            child: None,
            boundary: None,
        }
    }
}

impl<R: ?Sized, P: Default> Default for RenderNode<R, P> {
    fn default() -> Self {
        Self {
            parent_data: P::default(),

            needs_compositing: false,

            boundary: None,

            child: None,
        }
    }
}

impl<R: ?Sized, P> RenderNode<R, P> {
    /// Wires this node to `child`, replacing any previous one. A render object calls this from its
    /// `adopt_child`.
    #[allow(clippy::needless_pass_by_value)]
    pub fn set(&mut self, child: MountedChild<R>) {
        self.child = Some(child);

        // The handle changed, so a boundary registered against the old child would now resolve a different
        // node; drop it and let the next layout re-establish one for this child.
        self.boundary = None;
    }

    /// Drops this node's child, leaving it unwired. A boundary render object held past its child's unmount
    /// clears it so a later layout or paint finds no child rather than dereferencing the freed one.
    pub fn clear(&mut self) {
        self.child = None;
        self.boundary = None;
    }

    /// The deferred handle to the wired child, copied out, so a render object can register the child as a
    /// repaint boundary. Reached only after mount.
    ///
    /// # Safety
    /// The handle resolves a raw pointer to the child render object. The caller must resolve it only during a
    /// layout or paint pass, and must not keep it, or a boundary registered with it, past the child's unmount
    /// or replacement. `RenderNode` drops its own boundary in [`set`](Self::set) and [`clear`](Self::clear)
    /// for this reason.
    ///
    /// # Panics
    /// If the child is unwired (reached before mount).
    pub unsafe fn child_handle(&self) -> MountedChild<R> {
        *self
            .child
            .as_ref()
            .expect("child render node used before it was wired at mount")
    }

    /// Resolves the child's render object pointer, fresh, from its mounted address.
    fn resolve(&self) -> RenderObjectPtr<R> {
        let handle = self
            .child
            .as_ref()
            .expect("child render node used before it was wired at mount");

        // SAFETY: reached only during a pass, with no element hook on the stack; the resolver projects the
        // live child's address to its render object pointer, derived shared so it survives the child's rebuilds.
        unsafe { (handle.resolve)(handle.node) }
    }

    /// Borrows the child render object for one forwarding call, by shared reference. The typed forwarders
    /// below wrap this; reach for it directly only for a protocol they do not cover (a sliver, say).
    ///
    /// Borrow the child only during a layout or paint pass, with no element hook on the call stack.
    ///
    /// # Panics
    /// If the child is unwired (reached before mount). In debug, also if the render object is currently
    /// borrowed mutably.
    pub fn borrow(&self) -> RenderObjectRef<'_, R> {
        let inner = self.resolve().0;
        debug_mark_shared(inner);
        RenderObjectRef {
            inner,
            _marker: PhantomData,
        }
    }

    /// Borrows the child render object for one forwarding call, by exclusive reference. The typed forwarders
    /// below wrap this; reach for it directly only for a protocol they do not cover (a sliver, say).
    ///
    /// Borrow the child only during a layout or paint pass, with no element hook on the call stack.
    ///
    /// # Panics
    /// If the child is unwired (reached before mount). In debug, also if the render object is already
    /// borrowed, mutably or shared.
    pub fn borrow_mut(&mut self) -> RenderObjectMut<'_, R> {
        let inner = self.resolve().0;
        debug_mark_exclusive(inner);
        RenderObjectMut {
            inner,
            _marker: PhantomData,
        }
    }
}

/// In debug, registers a shared borrow of the render object behind `inner`, panicking if it is currently
/// borrowed exclusively. The matching [`RenderObjectRef`] releases it on drop. A no-op in release.
#[cfg_attr(not(debug_assertions), inline(always))]
fn debug_mark_shared<R: ?Sized>(inner: NonNull<RenderObjectInner<R>>) {
    #[cfg(debug_assertions)]
    {
        // SAFETY: the child cell is live and pinned for as long as the pass holds it.
        let borrows = unsafe { &(*inner.as_ptr()).borrows };
        assert!(
            borrows.get() >= 0,
            "render object reached for a shared borrow while it is borrowed mutably"
        );
        borrows.set(borrows.get() + 1);
    }
    let _ = inner;
}

/// In debug, registers an exclusive borrow of the render object behind `inner`, panicking if any borrow is
/// currently live. The matching [`RenderObjectMut`] releases it on drop. A no-op in release.
#[cfg_attr(not(debug_assertions), inline(always))]
fn debug_mark_exclusive<R: ?Sized>(inner: NonNull<RenderObjectInner<R>>) {
    #[cfg(debug_assertions)]
    {
        // SAFETY: the child cell is live and pinned for as long as the pass holds it.
        let borrows = unsafe { &(*inner.as_ptr()).borrows };
        assert!(
            borrows.get() == 0,
            "render object reached for an exclusive borrow while it is already borrowed"
        );
        borrows.set(-1);
    }
    let _ = inner;
}

impl<R: RenderObject + ?Sized, P> RenderNode<R, P> {
    /// Whether this child's subtree contributes a compositing layer, as of the last recompute.
    pub fn needs_compositing(&self) -> bool {
        self.needs_compositing
    }

    /// Records the child render object's subtree into `s`.
    pub fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        self.borrow_mut().build_semantics(s);
    }

    /// Captures the child render object's subtree, annotated with this holder's pipeline state.
    pub fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.decorate()
            .flag("needs_compositing", self.needs_compositing)
            .child(|d| self.borrow().describe(d))
    }
}

impl<R: RenderBox + ?Sized, P> RenderNode<R, P> {
    pub fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.borrow().min_intrinsic_width(height)
    }

    pub fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.borrow().max_intrinsic_width(height)
    }

    pub fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.borrow().min_intrinsic_height(width)
    }

    pub fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.borrow().max_intrinsic_height(width)
    }

    pub fn measure(&self, constraints: BoxConstraints) -> Size {
        self.borrow().measure(constraints)
    }

    /// Lays this child out under `constraints` without reading the size it takes, so the child is its own
    /// relayout boundary. If you need the size, use `layout_and_get_size`.
    pub fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) {
        self.layout_inner(ctx, constraints, false);
    }

    /// Lays this child out under `constraints` and returns the size it took. This couples the child with the
    /// parent, so a change to the child's size re-lays the parent too; the child is its own relayout boundary
    /// only when the constraints are tight, which fix its size whatever it does.
    pub fn layout_and_get_size(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
    ) -> Size {
        self.layout_inner(ctx, constraints, true)
    }

    fn layout_inner(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        parent_uses_size: bool,
    ) -> Size {
        // The child is a boundary when its size cannot affect the parent: under tight constraints, or when
        // the parent does not read its size.
        if constraints.is_tight() || !parent_uses_size {
            // Lay the child out under its own boundary, so its descendants mark it for re-layout rather than
            // the boundary enclosing it.
            let scope = self.register_boundary(ctx, constraints);

            ctx.with_layout_scope(scope, |ctx| self.borrow_mut().layout(ctx, constraints))
        } else {
            self.boundary = None;
            self.borrow_mut().layout(ctx, constraints)
        }
    }

    /// Registers the child as a relayout boundary, or refreshes the constraints of one already registered so a
    /// later isolated re-lay uses the latest. An existing boundary is updated in place: re-registering would
    /// hand out a new scope, stranding the one descendants captured.
    fn register_boundary(&mut self, ctx: &LayoutCtx, constraints: BoxConstraints) -> LayoutScope {
        let handle = *self
            .child
            .as_ref()
            .expect("child render node used before it was wired at mount");

        let boundary = Box::new(InlineBoxBoundary {
            handle,
            constraints,
        });

        if let Some(registered) = self.boundary.as_mut() {
            registered.replace(boundary);

            return registered.scope();
        }

        let boundary = ctx.register_layout_boundary(boundary);

        let scope = boundary.scope();

        self.boundary = Some(boundary);

        scope
    }

    pub fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.borrow().measure_baseline(constraints, baseline)
    }

    pub fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.borrow_mut().distance_to_baseline(baseline)
    }

    pub fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.borrow().hit_test(result, position)
    }

    pub fn update_compositing_bits(&mut self) -> bool {
        let needs_compositing = self.borrow_mut().update_compositing_bits();
        self.needs_compositing = needs_compositing;
        needs_compositing
    }

    pub fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        // If the child is a relayout boundary, record the repaint boundary enclosing it now that paint knows
        // it, so a later isolated re-lay repaints the right subtree.
        if let Some(registered) = self.boundary.as_mut() {
            registered.set_paint_scope(ctx.scope());
        }

        self.borrow_mut().paint(ctx, offset);
    }
}

use std::any::Any;

use typed_floats::{Positive, PositiveFinite};

pub use agui_core::render_object::{
    MountedChild, RenderObjectCell, RenderObjectMut, RenderObjectPtr, RenderObjectRef,
};

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
    },
    semantics::SemanticsTreeBuilder,
    text::TextBaseline,
};

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
/// parent keeps about it (its [`child_data`](Self::child_data) and pipeline flags).
///
/// The child render object is owned by the child's element, not here; this holds only the deferred handle to
/// it ([`MountedChild`]), [`set`](Self::set) once the child is attached. The handle is resolved fresh each
/// pass and the resulting render object is interior-mutable, so it survives the child's rebuilds and an
/// isolated relayout can reach it alongside a normal layout. Resolving is the one `unsafe` step, kept off the
/// render objects that hold the node: they call the safe forwarding methods below, which borrow the child for
/// one call.
pub struct RenderNode<R: ?Sized, P = ()> {
    pub child_data: P,

    needs_compositing: bool,

    /// Registered only while the child is its own relayout boundary; dropping it unregisters. The box layout
    /// forwarders set and clear it, and `set`/`clear` drop it so it never outlives the child it holds.
    boundary: Option<LayoutBoundaryHandle>,

    /// Deferred handle to the child render object, `None` until [`set`](Self::set) wires it at mount.
    child: Option<MountedChild<R>>,
}

impl<R: ?Sized, P: Default> RenderNode<R, P> {
    pub fn new(child_data: P) -> Self {
        Self {
            child_data,

            needs_compositing: false,

            child: None,
            boundary: None,
        }
    }
}

impl<R: ?Sized, P: Default> Default for RenderNode<R, P> {
    fn default() -> Self {
        Self {
            child_data: P::default(),

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
    /// The handle resolves a raw pointer to the child render object. Resolving it must not coexist with a
    /// borrow of the child element, per [`Element`](crate::element::Element)'s contract, and the handle, or a
    /// boundary registered with it, must not be kept past the child's unmount or replacement. `RenderNode`
    /// drops its own boundary in [`set`](Self::set) and [`clear`](Self::clear) for this reason.
    ///
    /// # Panics
    /// If the child is unwired (reached before mount).
    pub unsafe fn child_handle(&self) -> MountedChild<R> {
        *self.child()
    }

    /// The wired child handle, or a panic if reached before mount.
    fn child(&self) -> &MountedChild<R> {
        self.child
            .as_ref()
            .expect("child render node used before it was wired at mount")
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
        self.child().borrow()
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
        self.child().borrow_mut()
    }
}

impl<R: RenderObject + ?Sized, P> RenderNode<R, P> {
    /// Whether this child's subtree contributes a compositing layer, as of the last recompute.
    pub fn needs_compositing(&self) -> bool {
        self.needs_compositing
    }

    /// Returns the child render object's parent data, the layout configuration this node's parent reads to
    /// place it. Returns a value no downcast matches when the node is unwired.
    pub fn parent_data(&self) -> &dyn Any {
        match self.child.as_ref() {
            // SAFETY: a pass-time shared read bound to `&self`, so the borrow checker forbids a `borrow_mut`
            // of the same node while it is held, leaving no exclusive borrow to alias.
            Some(child) => unsafe { child.value_ref() }.parent_data(),
            None => &(),
        }
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
        let handle = *self.child();

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

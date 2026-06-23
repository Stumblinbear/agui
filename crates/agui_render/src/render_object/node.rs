use std::ptr::NonNull;

use typed_floats::{Positive, PositiveFinite};

use crate::{
    context::PaintCtx,
    diagnostics::{Diagnostics, DiagnosticsNode},
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    render_object::{
        LayoutCtx, RenderObject,
        box_layout::{BoxConstraints, RenderBox},
    },
    text::TextBaseline,
};

/// A just-mounted child's render object, handed back by [`UpdateCtx::mount`] for a render object to pass to
/// its `adopt_child`.
///
/// [`UpdateCtx::mount`]: crate::context::UpdateCtx::mount
pub struct MountedChild<R: ?Sized>(NonNull<R>);

impl<R: ?Sized> MountedChild<R> {
    /// Captures the just-mounted `child` render object as an edge to it.
    ///
    /// # Safety
    /// `child` must outlive the edge and stay at its address while held, and a layout or paint pass must reach
    /// it through this edge alone, so no other `&mut` to it is live.
    pub unsafe fn new(child: &mut R) -> Self {
        MountedChild(NonNull::from(child))
    }
}

/// A parent render object's edge to one child render object, paired with the per-child layout state the
/// parent keeps about it (its [`parent_data`](Self::parent_data) and pipeline flags).
///
/// The child render object is owned by the child's element, not here; this holds only a pointer to it,
/// [`set`](Self::set) once the child is mounted and pinned. Reaching the child is the one `unsafe` step,
/// kept off the render objects that hold the edge: they call the safe forwarding methods below.
pub struct RenderNode<R: ?Sized, P = ()> {
    pub parent_data: P,

    parent_uses_size: bool,
    needs_compositing: bool,

    /// Edge to the child render object, `None` until [`set`](Self::set) wires it at mount.
    child: Option<NonNull<R>>,
}

impl<R: ?Sized, P: Default> RenderNode<R, P> {
    pub fn new(parent_data: P) -> Self {
        Self {
            parent_data,

            parent_uses_size: false,
            needs_compositing: false,

            child: None,
        }
    }
}

impl<R: ?Sized, P: Default> Default for RenderNode<R, P> {
    fn default() -> Self {
        Self {
            parent_data: P::default(),

            parent_uses_size: false,
            needs_compositing: false,

            child: None,
        }
    }
}

impl<R: ?Sized, P> RenderNode<R, P> {
    /// Sets this node's edge to `child`, replacing any previous one. A render object calls this from its
    /// `adopt_child`.
    #[allow(clippy::needless_pass_by_value)]
    pub fn set(&mut self, child: MountedChild<R>) {
        self.child = Some(child.0);
    }

    /// Drops this node's edge, leaving it unwired. A boundary render object held past its child's unmount
    /// clears the edge so a later layout or paint finds no child rather than dereferencing the freed one.
    pub fn clear(&mut self) {
        self.child = None;
    }
}

impl<R: RenderObject + ?Sized, P> RenderNode<R, P> {
    /// Whether this child's subtree contributes a compositing layer, as of the last recompute.
    pub fn needs_compositing(&self) -> bool {
        self.needs_compositing
    }

    /// Captures the child render object's subtree, annotated with this holder's pipeline state.
    pub fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.decorate()
            .flag("parent_uses_size", self.parent_uses_size)
            .flag("needs_compositing", self.needs_compositing)
            .child(|d| self.as_ref().describe(d))
    }
}

impl<R: RenderBox + ?Sized, P> RenderNode<R, P> {
    pub fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.as_ref().min_intrinsic_width(height)
    }

    pub fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.as_ref().max_intrinsic_width(height)
    }

    pub fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.as_ref().min_intrinsic_height(width)
    }

    pub fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.as_ref().max_intrinsic_height(width)
    }

    pub fn measure(&self, constraints: BoxConstraints) -> Size {
        self.as_ref().measure(constraints)
    }

    /// Lays this child out under `constraints`. If you need the resulting size, use `layout_and_get_size`.
    pub fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) {
        self.as_mut().layout(ctx, constraints);
    }

    /// Lays this child out under `constraints` and returns the size it took. This couples the child with the
    /// parent, so a change to the child's size re-lays the parent too.
    pub fn layout_and_get_size(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
    ) -> Size {
        let size = self.as_mut().layout(ctx, constraints);
        self.parent_uses_size = true;
        size
    }

    pub fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.as_ref().measure_baseline(constraints, baseline)
    }

    pub fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.as_mut().distance_to_baseline(baseline)
    }

    pub fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.as_ref().hit_test(result, position)
    }

    pub fn update_compositing_bits(&mut self) -> bool {
        self.needs_compositing = self.as_mut().update_compositing_bits();
        self.needs_compositing
    }

    pub fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.as_mut().paint(ctx, offset);
    }
}

impl<R: ?Sized, P> AsRef<R> for RenderNode<R, P> {
    fn as_ref(&self) -> &R {
        let child = self
            .child
            .expect("child edge used before it was wired at mount");

        // SAFETY: `wire`'s contract: a live, pinned child reached only through this edge during the pass.
        unsafe { child.as_ref() }
    }
}

impl<R: ?Sized, P> AsMut<R> for RenderNode<R, P> {
    fn as_mut(&mut self) -> &mut R {
        let mut child = self
            .child
            .expect("child edge used before it was wired at mount");

        // SAFETY: as `child`, and `&mut self` rules out another borrow of the edge.
        unsafe { child.as_mut() }
    }
}

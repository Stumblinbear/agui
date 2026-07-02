mod any_element;

pub use any_element::*;

use crate::{
    context::{MessageCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    render_object::{RenderObject, RenderObjectCell, RenderObjectPtr},
};

/// A persistent node in the element tree, holding a widget's state and its child slots across rebuilds.
///
/// An element that reconciles more than one child wraps the walk in [`UpdateCtx::with_children`] so the
/// children's layout marks drain ancestor-before-descendant; debug builds assert this.
///
/// # Safety
/// An element co-maintains the tree and the addresses other components resolve from it as raw pointers, so a
/// wrong implementation causes undefined behavior in other safe code, not only its own: the parent that lays
/// out a child render object, a descendant reading a provided value, the driver dispatching a message. An
/// implementor must uphold:
///
/// - Each child is registered once through the [`UpdateCtx`] child operations and deregistered once when it
///   leaves. Leaving a child mounted, or unmounting it twice, corrupts the tree every other element walks.
/// - A child slot is reused in place only for a child that `can_update` the previous one (same type and key);
///   otherwise the old child is unmounted and the new one mounted. Reusing a slot across types is a
///   type-confused read of the render object.
/// - A render object's edge to a child is cleared when that child unmounts, so a later layout or paint
///   resolves no freed node.
/// - [`render_object_ptr`](Self::render_object_ptr) returns a pointer to this element's live render object,
///   valid for as long as the element is mounted. A render-less element (`Render = ()`) returns a placeholder,
///   sound only because that pointer is never dereferenced.
/// - This element owns its render object, and the render object of any child it holds inline. A borrow of the
///   element claims all of it. That borrow must never coexist with any access to a render object it owns: it is
///   undefined behavior to borrow the element while such a render object is being accessed, or to access one
///   while the element is borrowed. The element is borrowed only by its own lifecycle methods, so the two never
///   overlap.
pub unsafe trait Element {
    /// The render object this element owns and presents to its parent.
    type Render: ?Sized;

    /// A pointer to this element's render object, for a parent to hold and resolve each pass. A render-bearing
    /// element hands back its cell's; a transparent element forwards its child's; a render-less one returns a
    /// placeholder.
    fn render_object_ptr(&self) -> RenderObjectPtr<Self::Render>;

    /// Registers this element's children as it enters the tree.
    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        let _ = ctx;
    }

    /// Deregisters this element's children as it leaves the tree.
    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        let _ = ctx;
    }

    /// Re-runs this element's build, reconciling its children.
    fn rebuild(&mut self, ctx: &mut UpdateCtx<'_>) {
        let _ = ctx;
    }

    /// Responds to a provided value this element depends on changing, then rebuilds.
    fn dependency_changed(&mut self, ctx: &mut UpdateCtx<'_>) {
        let _ = ctx;
    }

    /// Handles a message delivered to this element.
    fn message(&mut self, ctx: &mut MessageCtx<'_>) {
        let _ = ctx;
    }

    /// Captures this element's subtree as a diagnostics snapshot.
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>().finish()
    }
}

/// A leaf element with no children, owning the render object its widget produced.
pub struct LeafElement<R> {
    render: RenderObjectCell<R>,
}

impl<R> LeafElement<R> {
    pub fn new(render: R) -> Self {
        Self {
            render: RenderObjectCell::new(render),
        }
    }

    /// This element's render object, by exclusive reference, for the widget's own writes during reconcile.
    pub fn render_object_mut(&mut self) -> &mut R {
        self.render.get_mut()
    }
}

// SAFETY: no children to register; `render_object_ptr` returns the cell's pointer, valid for the element's
// mounted life.
unsafe impl<R: RenderObject> Element for LeafElement<R> {
    type Render = R;

    fn render_object_ptr(&self) -> RenderObjectPtr<R> {
        self.render.render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.render.get_mut().attach(ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.render.get_mut().detach(ctx);
    }
}

mod any_element;
pub mod node;
mod shared;

pub use any_element::*;
pub use shared::*;

use crate::{
    context::{MessageCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    render_object::node::{RenderObjectCell, RenderObjectPtr},
};

/// A persistent node in the element tree, holding a widget's state and its child slots across rebuilds.
pub trait Element {
    /// The render object this element owns and presents to its parent.
    type Render: ?Sized;

    /// This element's render object, by exclusive reference, for the element's own writes (a widget updating
    /// its render props). A transparent element forwards to its child's.
    fn render_object_mut(&mut self) -> &mut Self::Render;

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

/// The leaf element with a unit render: no children, no state.
impl Element for () {
    type Render = ();

    fn render_object_mut(&mut self) -> &mut () {
        self
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<()> {
        RenderObjectPtr::dangling()
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
}

impl<R> Element for LeafElement<R> {
    type Render = R;

    fn render_object_mut(&mut self) -> &mut R {
        self.render.get_mut()
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<R> {
        self.render.render_object_ptr()
    }
}

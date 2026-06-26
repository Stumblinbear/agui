use crate::context::UpdateCtx;
use crate::diagnostics::{Diagnostics, DiagnosticsNode};
use crate::semantics::SemanticsTreeBuilder;

// Render-object modules reach the layout context through this module.
pub(crate) use crate::context::LayoutCtx;

mod any_render_object;
pub mod box_layout;
mod children;
pub mod node;
pub mod sliver;

pub use any_render_object::*;
pub use children::*;

/// An object in the render tree.
///
/// A [`RenderObject`] has a lifecycle but does not itself define a coordinate system or layout
/// protocol. Those are introduced by the traits that extend it: [`RenderBox`], which lays out in
/// Cartesian coordinates, and [`RenderSliver`](crate::render_object::sliver::RenderSliver), which
/// lays out along a scroll axis.
pub trait RenderObject: 'static {
    /// Called as this render object enters the live render tree.
    fn attach(&mut self, ctx: &mut UpdateCtx<'_>) {
        let _ = ctx;
    }

    /// Called as this render object leaves the render tree, but does not guarantee it will be subsequently
    /// dropped. It may still re-attach to the tree before the end of the frame, and may be re-attached in a
    /// new location.
    fn detach(&mut self, ctx: &mut UpdateCtx<'_>) {
        let _ = ctx;
    }

    /// Adds this render object's own semantic node, if any, to `s`, then recurses into each child.
    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>);

    /// Captures this render object's subtree as a diagnostics snapshot, recursing into each child.
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode;
}

impl RenderObject for () {
    fn build_semantics(&mut self, _s: &mut SemanticsTreeBuilder<'_>) {}

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>().finish()
    }
}

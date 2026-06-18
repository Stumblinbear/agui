use crate::{
    context::{LayoutCtx, MountCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
};

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
    fn mount(&mut self, ctx: &mut MountCtx);

    fn unmount(&mut self, ctx: &mut MountCtx);

    /// Captures this render object's subtree as a diagnostics snapshot.
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>().finish()
    }
}

impl RenderObject for () {
    fn mount(&mut self, _: &mut MountCtx) {}

    fn unmount(&mut self, _: &mut MountCtx) {}
}

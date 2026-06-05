use crate::context::{LayoutCtx, MountCtx};

mod any_render_object;
pub mod box_layout;
pub mod node;
pub mod sliver;

pub use any_render_object::*;

/// An object in the render tree.
///
/// A [`RenderObject`] has a lifecycle but does not itself define a coordinate system or layout
/// protocol. Those are introduced by the traits that extend it: [`RenderBox`], which lays out in
/// Cartesian coordinates, and [`RenderSliver`](crate::render_object::sliver::RenderSliver), which
/// lays out along a scroll axis.
pub trait RenderObject: 'static {
    fn mount(&mut self, ctx: &mut MountCtx);

    fn unmount(&mut self, ctx: &mut MountCtx);

    /// Recomputes whether this subtree contributes a compositing layer, and returns it. An
    /// implementation must fold in every child's bit; a node that reads its own bit while painting
    /// caches it here.
    fn update_compositing_bits(&mut self) -> bool;
}

impl RenderObject for () {
    fn mount(&mut self, _: &mut MountCtx) {}

    fn unmount(&mut self, _: &mut MountCtx) {}

    fn update_compositing_bits(&mut self) -> bool {
        false
    }
}

use std::any::Any;

use crate::context::UpdateCtx;
use crate::diagnostics::{Diagnostics, DiagnosticsNode};

/// An object in the render tree.
///
/// A `RenderObject` has a lifecycle but does not itself define a coordinate system or layout protocol.
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

    /// Returns this render object's parent data, the layout configuration its parent reads to place it. The
    /// parent downcasts the result to the type it expects. A render object that carries no parent data returns
    /// a value no such downcast matches.
    fn parent_data(&self) -> &dyn Any {
        &()
    }

    /// Captures this render object's subtree as a diagnostics snapshot, recursing into each child.
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode;
}

impl RenderObject for () {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>().finish()
    }
}

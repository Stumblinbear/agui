use std::any::Any;
use std::rc::Rc;

use agui_core::tree::NodeHandle;

use crate::context::BuildCtx;
use crate::paint::compositing::{LayerHandle, OffsetLayer};
use crate::pipeline::BoundaryContent;
use crate::pipeline::render_pipeline::{
    LayoutBoundary, LayoutBoundaryHandle, LayoutScope, PaintBoundaryHandle, PaintScope,
    RenderPipeline,
};
use crate::provide::ProvideScope;

/// The context passed to a widget's `create`: the values in scope and the render pipeline, but no cursor.
/// `create` builds the element and its render object before the tree exists, so it registers no element-tree
/// node. It may plant a render tree in the forest: a [`View`](crate::view::View) registers its boundaries
/// here. Reconcile-time grafts derive one from an [`UpdateCtx`](crate::context::UpdateCtx).
pub struct CreateCtx {
    provide: ProvideScope,
    pipeline: RenderPipeline,
}

impl CreateCtx {
    /// A create context in `scope` against `pipeline`. The driver builds one to create the root widget.
    pub fn new(scope: ProvideScope, pipeline: RenderPipeline) -> Self {
        Self {
            provide: scope,
            pipeline,
        }
    }

    /// The nearest provided value of type `T` in scope, or `None`, without recording a dependency.
    pub fn get_provided<T: Any>(&self) -> Option<Rc<T>> {
        self.provide.get::<T>()
    }

    /// Runs `f` with the widget-facing [`BuildCtx`] for this scope, for a stateful widget to compose its
    /// child during `create`. No element handle exists yet, so a dependency read cannot register until mount.
    /// The context lives only for the call.
    pub fn build<R>(&self, f: impl FnOnce(&mut BuildCtx) -> R) -> R {
        f(&mut BuildCtx::new(self.provide, NodeHandle::default()))
    }

    /// Registers `boundary` as the relayout boundary at the root of a view's render tree, and returns the
    /// handle that owns it. A [`View`](crate::view::View) plants its tree's relayout root this way at `create`;
    /// it is a forest root, so its enclosing layout scope is detached. Its enclosing repaint boundary is
    /// recorded through [`set_paint_scope`](crate::pipeline::render_pipeline::LayoutBoundaryHandle::set_paint_scope).
    pub fn register_layout_boundary(
        &self,
        boundary: Box<dyn LayoutBoundary>,
    ) -> LayoutBoundaryHandle {
        self.pipeline
            .register_layout_boundary(LayoutScope::detached(), boundary)
    }

    /// Registers `content` as the repaint boundary at the root of a view's render tree, painting into
    /// `layer`, and returns the handle that owns it. A [`View`](crate::view::View) plants its tree's repaint
    /// root this way at `create`; it is a forest root, so its enclosing paint scope is detached.
    pub fn register_paint_boundary(
        &self,
        content: BoundaryContent,
        layer: LayerHandle<OffsetLayer>,
    ) -> PaintBoundaryHandle {
        self.pipeline
            .register_paint_boundary(PaintScope::detached(), content, layer)
    }
}

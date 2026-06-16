use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    paint::compositing::{ExternalSurfaceId, ExternalSurfaceLayer, LayerHandle},
    prelude::{element::*, render_object::*},
};

/// A widget that fills its bounds with a surface the system compositor owns, rather than
/// rasterizing anything itself.
///
/// The surface is identified by an [`ExternalSurfaceId`] a driver mints for a surface it manages,
/// such as decoded video; composing reports the surface's bounds back to the driver so it can
/// position the matching system visual. The widget is output-only: it draws nothing and absorbs no
/// input.
///
/// It fills the constraints it is given, so place it under a sizing parent, or set
/// [`width`](Self::width)/[`height`](Self::height) to size it directly.
pub struct ExternalSurface {
    surface: ExternalSurfaceId,
    width: Option<Positive<f32>>,
    height: Option<Positive<f32>>,
}

impl ExternalSurface {
    pub fn new(surface: ExternalSurfaceId) -> Self {
        Self {
            surface,
            width: None,
            height: None,
        }
    }

    pub fn width<T>(mut self, width: T) -> Self
    where
        PositiveFinite<f32>: TryFrom<T>,
        <PositiveFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        self.width = Some(
            PositiveFinite::try_from(width)
                .expect("invalid width given to ExternalSurface")
                .into(),
        );
        self
    }

    pub fn height<T>(mut self, height: T) -> Self
    where
        PositiveFinite<f32>: TryFrom<T>,
        <PositiveFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        self.height = Some(
            PositiveFinite::try_from(height)
                .expect("invalid height given to ExternalSurface")
                .into(),
        );
        self
    }
}

impl Widget for ExternalSurface {
    type Element = LeafElement<RenderExternalSurface>;

    type Render = RenderExternalSurface;

    fn create(self, _: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        (
            LeafElement::new(),
            RenderExternalSurface {
                surface: self.surface,
                width: self.width,
                height: self.height,

                layer: LayerHandle::new(ExternalSurfaceLayer::new(self.surface, Size::ZERO)),

                paint_scope: PaintScope::detached(),
                layout_scope: LayoutScope::detached(),
            },
        )
    }

    fn update(self, _: &mut Self::Element, render_object: &mut Self::Render, ctx: &mut UpdateCtx) {
        if render_object.surface != self.surface {
            render_object.surface = self.surface;
            render_object.layer.borrow_mut().set_surface(self.surface);
            ctx.mark_needs_paint(render_object.paint_scope);
        }

        if render_object.width != self.width || render_object.height != self.height {
            render_object.width = self.width;
            render_object.height = self.height;
            ctx.mark_needs_layout(render_object.layout_scope);
        }
    }
}

pub struct RenderExternalSurface {
    surface: ExternalSurfaceId,
    width: Option<Positive<f32>>,
    height: Option<Positive<f32>>,

    layer: LayerHandle<ExternalSurfaceLayer>,

    paint_scope: PaintScope,
    layout_scope: LayoutScope,
}

impl RenderObject for RenderExternalSurface {
    fn mount(&mut self, ctx: &mut MountCtx) {
        self.paint_scope = *ctx.paint_scope();
    }

    fn unmount(&mut self, _: &mut MountCtx) {}

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property("surface", self.surface.0)
            .property_opt("width", self.width)
            .property_opt("height", self.height)
            .finish()
    }
}

impl RenderBox for RenderExternalSurface {
    fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.width?).ok()
    }

    fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.width?).ok()
    }

    fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.height?).ok()
    }

    fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.height?).ok()
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        BoxConstraints::tight_for(self.width, self.height)
            .enforce(constraints)
            .biggest()
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.layout_scope = *ctx.scope();

        let size = BoxConstraints::tight_for(self.width, self.height)
            .enforce(constraints)
            .biggest();
        self.layer.borrow_mut().set_size(size);

        size
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
        HitTest::Pass
    }

    fn update_compositing_bits(&mut self) -> bool {
        // A placed surface is its own compositing boundary, so ancestors composite it as a layer.
        true
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        ctx.add_layer(self.layer.clone(), offset);
    }
}

#[cfg(test)]
mod tests {
    use agui_core::{
        paint::compositing::{CompositedNode, Compositor, LayerHandle, OffsetLayer},
        prelude::{element::*, render_object::*},
        test_harness::TestCtx,
    };
    use agui_test::ElementLifecycleCheck;

    use super::*;

    #[test]
    fn obeys_the_element_lifecycle() {
        ElementLifecycleCheck::new().leaf(|| ExternalSurface::new(ExternalSurfaceId(42)));
    }

    #[test]
    fn places_a_surface_filling_its_bounds() {
        let widget = ExternalSurface::new(ExternalSurfaceId(42));
        let mut render = TestCtx::new().laid_out(widget, BoxConstraints::new(0, 64, 0, 48));

        let root = LayerHandle::new(OffsetLayer::new());
        PaintCtx::paint(&root, |ctx| render.paint(ctx, Offset::ZERO));
        let frame = Compositor::compose(&root);

        match frame.nodes() {
            [CompositedNode::External { surface, size, .. }] => {
                assert_eq!(*surface, ExternalSurfaceId(42));
                assert_eq!(*size, Size::new(64, 48));
            }
            other => panic!("expected one external node, got {other:?}"),
        }
    }

    #[test]
    fn an_explicit_size_overrides_filling() {
        let widget = ExternalSurface::new(ExternalSurfaceId(1))
            .width(20)
            .height(10);
        let mut render = TestCtx::new().laid_out(widget, BoxConstraints::new(0, 100, 0, 100));

        let root = LayerHandle::new(OffsetLayer::new());
        PaintCtx::paint(&root, |ctx| render.paint(ctx, Offset::ZERO));
        let frame = Compositor::compose(&root);

        match frame.nodes() {
            [CompositedNode::External { size, .. }] => {
                assert_eq!(*size, Size::new(20, 10));
            }
            other => panic!("expected one external node, got {other:?}"),
        }
    }
}

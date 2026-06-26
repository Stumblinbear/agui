use typed_floats::{Positive, PositiveFinite};

use crate::{
    paint::compositing::{ExternalSurfaceId, ExternalSurfaceLayer, LayerHandle},
    prelude::{element::*, render_object::*},
};

/// A widget that fills its bounds with a surface the system compositor owns, rather than rasterizing anything
/// itself.
///
/// The surface is identified by an [`ExternalSurfaceId`] a driver mints for a surface it manages, such as
/// decoded video; composing reports the surface's bounds back to the driver so it can position the matching
/// system visual. The widget is output-only: it draws nothing and absorbs no input.
///
/// It fills the constraints it is given, so place it under a sizing parent, or set [`width`](Self::width) /
/// [`height`](Self::height) to size it directly.
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

    /// # Panics
    /// If `width` is not a finite, non-negative size.
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

    /// # Panics
    /// If `height` is not a finite, non-negative size.
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

    fn create(self, _ctx: &mut CreateCtx) -> Self::Element {
        LeafElement::new(RenderExternalSurface {
            surface: self.surface,
            width: self.width,
            height: self.height,

            layer: LayerHandle::new(ExternalSurfaceLayer::new(self.surface, Size::ZERO)),

            paint_scope: PaintScope::detached(),
            layout_scope: LayoutScope::detached(),
        })
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let render = element.render_object_mut();

        if render.surface != self.surface {
            render.surface = self.surface;
            render.layer.borrow_mut().set_surface(self.surface);
            ctx.mark_needs_paint(render.paint_scope);
        }

        if render.width != self.width || render.height != self.height {
            render.width = self.width;
            render.height = self.height;
            ctx.mark_needs_layout(render.layout_scope);
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
    fn build_semantics(&mut self, _s: &mut SemanticsTreeBuilder<'_>) {}

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
        self.paint_scope = ctx.scope();
        ctx.add_layer(self.layer.clone(), offset);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        paint::compositing::{CompositedNode, ExternalSurfaceId},
        prelude::render_object::*,
        test_harness::TestCtx,
    };

    use super::ExternalSurface;

    /// The composited size and surface id of the one external node in a view's frame.
    fn external_node(view: &crate::view::ViewHandle) -> (ExternalSurfaceId, Size) {
        view.composite_frame()
            .nodes()
            .iter()
            .find_map(|node| match node {
                CompositedNode::External { surface, size, .. } => Some((*surface, *size)),
                _ => None,
            })
            .expect("an external surface node")
    }

    #[test]
    fn places_a_surface_filling_its_bounds() {
        let (mut owner, view) =
            TestCtx::new().mount_view(ExternalSurface::new(ExternalSurfaceId(42)));
        view.resize(BoxConstraints::new(0, 64, 0, 48));
        owner.flush_layout();
        owner.flush_paint();

        let (surface, size) = external_node(&view);
        assert_eq!(surface, ExternalSurfaceId(42));
        assert_eq!(size, Size::new(64, 48), "fills the constraints it is given");
    }

    #[test]
    fn an_explicit_size_overrides_filling() {
        let (mut owner, view) = TestCtx::new().mount_view(
            ExternalSurface::new(ExternalSurfaceId(1))
                .width(20)
                .height(10),
        );
        view.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();
        owner.flush_paint();

        let (_, size) = external_node(&view);
        assert_eq!(size, Size::new(20, 10), "takes its explicit size");
    }
}

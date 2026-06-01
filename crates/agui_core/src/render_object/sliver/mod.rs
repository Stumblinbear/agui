use typed_floats::{NonNaNFinite, Positive, PositiveFinite, as_const};

use crate::{
    axis::Axis,
    constraints::Constraints,
    context::UpdateCtx,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    render_object::{RenderNode, RenderObject, box_layout::BoxLayout},
    renderer::Canvas,
    size::Size,
    text_baseline::TextBaseline,
};

mod any_render_sliver;

pub use any_render_sliver::*;

/// Immutable layout constraints for a [`RenderSliver`].
///
/// Carries the viewport's current scroll state and the space available to the sliver when
/// [`SliverLayout::layout`] runs.
#[derive(Debug, Clone, Copy)]
pub struct SliverConstraints {
    /// The main axis the sliver is laid out along.
    pub axis: Axis,

    /// How far the leading edge of this sliver has been scrolled past the viewport's leading edge.
    pub scroll_offset: PositiveFinite<f32>,

    /// The total scroll extent of all slivers preceding this one.
    pub preceding_scroll_extent: PositiveFinite<f32>,

    /// Paintable main-axis space remaining in the viewport.
    pub remaining_paint_extent: Positive<f32>,

    /// The fixed extent available on the cross axis.
    pub cross_axis_extent: PositiveFinite<f32>,

    /// The viewport's full extent along the main axis.
    pub viewport_main_axis_extent: PositiveFinite<f32>,

    /// Cache space remaining beyond the visible area.
    pub remaining_cache_extent: Positive<f32>,
}

/// Describes the amount of space occupied by a [`RenderSliver`].
///
/// A sliver can occupy space in several different ways, which is why this type has multiple fields.
#[derive(Debug, Clone, Copy)]
pub struct SliverGeometry {
    /// The total main-axis extent the sliver can scroll through. Lazily built content reports an
    /// infinite extent.
    pub scroll_extent: Positive<f32>,

    /// The number of main-axis pixels the sliver paints within the viewport.
    pub paint_extent: PositiveFinite<f32>,

    /// The number of main-axis pixels the sliver consumes in the viewport's own layout. Must not
    /// exceed [`paint_extent`](Self::paint_extent).
    pub layout_extent: PositiveFinite<f32>,

    /// The largest `paint_extent` the sliver could produce given unlimited room.
    pub max_paint_extent: Positive<f32>,

    /// The main-axis distance from the sliver's layout position to where it begins painting. A
    /// pinned header that paints before its scroll position reports a negative origin.
    pub paint_origin: NonNaNFinite<f32>,

    /// The main-axis extent over which the sliver responds to hit tests.
    pub hit_test_extent: PositiveFinite<f32>,

    /// Whether the sliver has any visible content to paint.
    pub visible: bool,

    /// Whether the sliver paints content beyond its `paint_extent`.
    pub has_visual_overflow: bool,

    /// The main-axis extent the sliver occupies within the viewport's cache area.
    pub cache_extent: PositiveFinite<f32>,
}

impl SliverGeometry {
    /// Geometry for a sliver `scroll_extent` long that paints `paint_extent` of itself. The layout,
    /// hit-test, and cache extents follow `paint_extent`, and `paint_origin` is zero.
    pub fn new(scroll_extent: f32, paint_extent: f32) -> Self {
        let scroll_extent = Positive::try_from(scroll_extent).expect("scroll_extent must be >= 0");
        let paint_extent =
            PositiveFinite::try_from(paint_extent).expect("paint_extent must be finite and >= 0");
        let max_paint_extent =
            Positive::try_from(paint_extent.get()).expect("paint_extent must be >= 0");

        Self {
            scroll_extent,
            paint_extent,
            layout_extent: paint_extent,
            max_paint_extent,
            paint_origin: as_const!(NonNaNFinite, f32, 0.0),
            hit_test_extent: paint_extent,
            visible: paint_extent.get() > 0.0,
            has_visual_overflow: false,
            cache_extent: paint_extent,
        }
    }
}

/// The layout protocol for slivers. A sliver is laid out from [`SliverConstraints`] and reports
/// [`SliverGeometry`].
pub trait SliverLayout {
    fn layout(&mut self, constraints: SliverConstraints) -> SliverGeometry;
}

#[diagnostic::on_unimplemented(
    message = "Trait bound RenderSliver is not satisfied.",
    note = "RenderObject + SliverLayout is required to implement RenderSliver."
)]
pub trait RenderSliver: RenderObject + SliverLayout {}

impl<T> RenderSliver for T where T: RenderObject + SliverLayout {}

/// A box render object that hosts a single sliver and lays it out along the vertical axis. Requires
/// bounded main-axis constraints.
pub struct RenderViewport<S: RenderSliver> {
    /// The main-axis (vertical) scroll offset.
    pub offset: PositiveFinite<f32>,
    pub sliver: RenderNode<S>,
    geometry: Option<SliverGeometry>,
}

impl<S: RenderSliver> RenderViewport<S> {
    pub fn new(sliver: RenderNode<S>) -> Self {
        Self {
            offset: as_const!(PositiveFinite, f32, 0.0),
            sliver,
            geometry: None,
        }
    }

    pub fn geometry(&self) -> Option<SliverGeometry> {
        self.geometry
    }
}

impl<S: RenderSliver> RenderObject for RenderViewport<S> {
    fn mount(&mut self, _: &mut UpdateCtx) {}

    fn unmount(&mut self, _: &mut UpdateCtx) {}

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.sliver.object.hit_test(result, position)
    }

    fn paint(&mut self, canvas: &mut Canvas) {
        self.sliver.object.paint(canvas);
    }
}

impl<S: RenderSliver> BoxLayout for RenderViewport<S> {
    fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }
    fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }
    fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }
    fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn measure(&self, constraints: Constraints) -> Size {
        constraints.biggest()
    }

    fn layout(&mut self, constraints: Constraints) -> Size {
        let size = constraints.biggest();

        // Vertical main axis: main extent is height, cross extent is width.
        let main = size.height.get();
        let cross = size.width.get();

        let constraints = SliverConstraints {
            axis: Axis::Vertical,
            scroll_offset: self.offset,
            preceding_scroll_extent: as_const!(PositiveFinite, f32, 0.0),
            remaining_paint_extent: Positive::try_from(main).expect("viewport main extent >= 0"),
            cross_axis_extent: PositiveFinite::try_from(cross)
                .expect("viewport cross extent must be finite"),
            viewport_main_axis_extent: PositiveFinite::try_from(main)
                .expect("viewport main extent must be finite"),
            remaining_cache_extent: Positive::try_from(main).expect("viewport main extent >= 0"),
        };

        self.geometry = Some(SliverLayout::layout(&mut self.sliver.object, constraints));

        size
    }

    fn measure_baseline(&self, _: Constraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{element::Element, test_harness::TestHarness, view::AsAnyView, view::View};

    /// A sliver that occupies a fixed scroll extent and paints whatever of it is currently in view.
    struct RenderSliverFixed {
        extent: f32,
    }

    impl RenderObject for RenderSliverFixed {
        fn mount(&mut self, _: &mut UpdateCtx) {}

        fn unmount(&mut self, _: &mut UpdateCtx) {}

        fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
            HitTest::Pass
        }

        fn paint(&mut self, _: &mut Canvas) {}
    }

    impl SliverLayout for RenderSliverFixed {
        fn layout(&mut self, constraints: SliverConstraints) -> SliverGeometry {
            let remaining = constraints.remaining_paint_extent.get();
            let visible = (self.extent - constraints.scroll_offset.get()).clamp(0.0, remaining);
            SliverGeometry::new(self.extent, visible)
        }
    }

    #[test]
    fn viewport_drives_a_single_sliver() {
        let mut viewport =
            RenderViewport::new(RenderNode::new(RenderSliverFixed { extent: 100.0 }));

        // 100 wide x 50 tall viewport: the sliver is 100 long, only 50 fits.
        let size = BoxLayout::layout(&mut viewport, Constraints::tight(Size::new(100.0, 50.0)));
        assert_eq!(size, Size::new(100.0, 50.0));
        let g = viewport.geometry().unwrap();
        assert_eq!(g.scroll_extent.get(), 100.0);
        assert_eq!(
            g.paint_extent.get(),
            50.0,
            "clamped to the remaining paint extent"
        );

        // Scroll down 80 -> only the last 20 of the 100-long sliver remains visible.
        viewport.offset = PositiveFinite::try_from(80.0).unwrap();
        BoxLayout::layout(&mut viewport, Constraints::tight(Size::new(100.0, 50.0)));
        assert_eq!(viewport.geometry().unwrap().paint_extent.get(), 20.0);
    }

    /// A view whose render object is a sliver — to exercise the erased boundary.
    struct SliverFixedView {
        extent: f32,
    }

    impl View for SliverFixedView {
        type State = ();
        type Render = RenderSliverFixed;

        fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (vec![], ())
        }

        fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderSliverFixed {
                extent: self.extent,
            }
        }

        fn update_render_object(&self, _: &Element, object: &mut Self::Render) {
            object.extent = self.extent;
        }
    }

    #[test]
    fn viewport_drives_an_erased_sliver() {
        // into_boxed_render_sliver erases to Box<dyn AnyRenderSliver>, which is itself a RenderSliver,
        // so the viewport drives it identically to a concrete sliver.
        let boxed_view = SliverFixedView { extent: 100.0 }.into_boxed_render_sliver();
        let harness = TestHarness::mount(&boxed_view);

        let erased: Box<dyn AnyRenderSliver> = boxed_view.create_render_object(&harness.root);

        let mut viewport = RenderViewport::new(RenderNode::new(erased));
        BoxLayout::layout(&mut viewport, Constraints::tight(Size::new(100.0, 50.0)));

        assert_eq!(viewport.geometry().unwrap().paint_extent.get(), 50.0);
    }
}

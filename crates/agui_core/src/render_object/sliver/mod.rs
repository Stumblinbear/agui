use typed_floats::{NonNaNFinite, Positive, PositiveFinite, as_const};

use crate::{
    context::PaintCtx,
    diagnostics::{Diagnostics, DiagnosticsNode, ProtocolTag},
    geometry::{Axis, AxisDirection, Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    render_object::{
        LayoutCtx, MountCtx, RenderObject,
        box_layout::{BoxConstraints, RenderBox},
        node::RenderNode,
    },
    text::TextBaseline,
};

mod any_render_sliver;

pub use any_render_sliver::*;

/// The direction in which a sliver's contents are ordered, relative to the [`AxisDirection`].
///
/// [`Forward`](GrowthDirection::Forward) orders contents along the axis direction;
/// [`Reverse`](GrowthDirection::Reverse) orders them against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrowthDirection {
    Forward,
    Reverse,
}

/// The direction the user is currently scrolling, relative to the [`AxisDirection`], or
/// [`Idle`](ScrollDirection::Idle) when no scroll is in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollDirection {
    Idle,
    Forward,
    Reverse,
}

/// Immutable layout constraints for a [`RenderSliver`].
///
/// Carries the viewport's current scroll state and the space available to the sliver when
/// [`RenderSliver::layout`] runs.
#[derive(Debug, Clone, Copy)]
pub struct SliverConstraints {
    /// The direction of the sliver's main axis.
    ///
    /// [`scroll_offset`](Self::scroll_offset) and the paint and cache extents increase in this
    /// direction.
    pub axis_direction: AxisDirection,

    /// The direction in which the cross axis increases, orthogonal to
    /// [`axis_direction`](Self::axis_direction).
    pub cross_axis_direction: AxisDirection,

    /// The sliver's [`GrowthDirection`] relative to [`axis_direction`](Self::axis_direction).
    pub growth_direction: GrowthDirection,

    /// The user's current [`ScrollDirection`].
    pub user_scroll_direction: ScrollDirection,

    /// How far the leading edge of this sliver has been scrolled past the viewport's leading edge.
    pub scroll_offset: PositiveFinite<f32>,

    /// The total scroll extent of all slivers preceding this one. Infinite when one of them has an
    /// infinite extent.
    pub preceding_scroll_extent: Positive<f32>,

    /// The number of pixels by which preceding pinned or floating slivers overlap this one.
    pub overlap: NonNaNFinite<f32>,

    /// Paintable main-axis space remaining in the viewport.
    pub remaining_paint_extent: Positive<f32>,

    /// The fixed extent available on the cross axis.
    pub cross_axis_extent: PositiveFinite<f32>,

    /// The viewport's full extent along the main axis.
    pub viewport_main_axis_extent: PositiveFinite<f32>,

    /// Cache space remaining beyond the visible area.
    pub remaining_cache_extent: Positive<f32>,
}

impl SliverConstraints {
    /// The main [`Axis`] this sliver is laid out along.
    pub fn axis(&self) -> Axis {
        self.axis_direction.axis()
    }
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

    /// The amount by which the viewport should shift its scroll offset before laying out again, or
    /// [`None`] to request no correction.
    ///
    /// When set, the rest of this geometry is ignored: the viewport applies the correction and
    /// reruns layout. A sliver returning a correction need not compute the other fields.
    pub scroll_offset_correction: Option<NonNaNFinite<f32>>,
}

impl SliverGeometry {
    /// Geometry for a sliver `scroll_extent` long that paints `paint_extent` of itself. The layout,
    /// hit-test, and cache extents follow `paint_extent`, and `paint_origin` is zero.
    ///
    /// # Panics
    ///
    /// Panics if `scroll_extent` is negative, or if `paint_extent` is negative or infinite.
    pub fn new<S, P>(scroll_extent: S, paint_extent: P) -> Self
    where
        Positive<f32>: TryFrom<S>,
        <Positive<f32> as TryFrom<S>>::Error: std::fmt::Debug,
        PositiveFinite<f32>: TryFrom<P>,
        <PositiveFinite<f32> as TryFrom<P>>::Error: std::fmt::Debug,
    {
        let scroll_extent = Positive::try_from(scroll_extent).expect("scroll_extent must be >= 0");
        let paint_extent =
            PositiveFinite::try_from(paint_extent).expect("paint_extent must be finite and >= 0");
        let max_paint_extent = <Positive<f32> as TryFrom<f32>>::try_from(paint_extent.get())
            .expect("paint_extent must be >= 0");

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
            scroll_offset_correction: None,
        }
    }
}

/// A render object that occupies a portion of a scrolling viewport.
///
/// A [`RenderViewport`] lays its slivers out one after another along the scroll axis. Each is laid
/// out against the viewport's current scroll state ([`SliverConstraints`]) and reports the space it
/// occupies as [`SliverGeometry`].
pub trait RenderSliver: RenderObject {
    fn layout(&mut self, constraints: SliverConstraints) -> SliverGeometry;

    /// Determines the set of sliver render objects located at the given position, in the sliver's
    /// own axis-relative space: `main_axis_position` runs along the scroll axis from the sliver's
    /// leading painted edge, `cross_axis_position` along the cross axis.
    ///
    /// Returns [`HitTest::Absorb`], and adds any render objects that contain the point to `result`,
    /// if this sliver or one of its descendants absorbs the hit (preventing render objects below
    /// this one from being hit). Returns [`HitTest::Pass`] if the hit can continue to render objects
    /// below this one.
    ///
    /// Hit testing requires layout to be up to date but not paint: an implementation may rely on
    /// [`RenderSliver::layout`] having been called, but not on [`RenderSliver::paint`].
    fn hit_test(
        &self,
        result: &mut HitTestResult,
        main_axis_position: PositiveFinite<f32>,
        cross_axis_position: PositiveFinite<f32>,
    ) -> HitTest;

    /// Paints this sliver and its descendants. `offset` is the sliver's paint origin in the enclosing
    /// boundary's layer coordinate space.
    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset);
}

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
    fn mount(&mut self, _: &mut MountCtx) {}

    fn unmount(&mut self, _: &mut MountCtx) {}

    fn update_compositing_bits(&mut self) -> bool {
        self.sliver.update_compositing_bits()
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child_in(Some(ProtocolTag::SLIVER), |d| self.sliver.describe(d))
            .finish()
    }
}

impl<S: RenderSliver> RenderBox for RenderViewport<S> {
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

    fn measure(&self, constraints: BoxConstraints) -> Size {
        constraints.biggest()
    }

    // TODO(trevin): hook up slivers to layout
    fn layout(&mut self, _: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        let size = constraints.biggest();

        // Vertical main axis: main extent is height, cross extent is width.
        let main = size.height.get();
        let cross = size.width.get();

        let constraints = SliverConstraints {
            axis_direction: AxisDirection::Down,
            cross_axis_direction: AxisDirection::Right,
            growth_direction: GrowthDirection::Forward,
            user_scroll_direction: ScrollDirection::Idle,
            scroll_offset: self.offset,
            preceding_scroll_extent: as_const!(Positive, f32, 0.0),
            overlap: as_const!(NonNaNFinite, f32, 0.0),
            remaining_paint_extent: Positive::try_from(main).expect("viewport main extent >= 0"),
            cross_axis_extent: PositiveFinite::try_from(cross)
                .expect("viewport cross extent must be finite"),
            viewport_main_axis_extent: PositiveFinite::try_from(main)
                .expect("viewport main extent must be finite"),
            remaining_cache_extent: Positive::try_from(main).expect("viewport main extent >= 0"),
        };

        self.geometry = Some(RenderSliver::layout(&mut self.sliver.object, constraints));

        size
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        // Vertical-forward: y maps to the main axis, x to the cross axis. The position is already
        // viewport-local; only forward hits that land within the viewport (non-negative).
        let (main, cross) = (position.y.get(), position.x.get());

        if main < 0.0 || cross < 0.0 {
            return HitTest::Pass;
        }

        RenderSliver::hit_test(
            &self.sliver.object,
            result,
            PositiveFinite::try_from(main).expect("main-axis position >= 0"),
            PositiveFinite::try_from(cross).expect("cross-axis position >= 0"),
        )
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.sliver.object.paint(ctx, offset);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;
    use crate::{
        context::UpdateCtx,
        element::Element,
        pipeline::{layout::LayoutPipeline, paint::PaintPipeline},
        prelude::render_object::LayoutScope,
        test_harness::TestCtx,
        widget::{AsAnyWidget, Widget},
    };

    /// A sliver that occupies a fixed scroll extent and paints whatever of it is currently in widget.
    struct RenderSliverFixed {
        extent: f32,
    }

    impl RenderObject for RenderSliverFixed {
        fn mount(&mut self, _: &mut MountCtx) {}

        fn unmount(&mut self, _: &mut MountCtx) {}

        fn update_compositing_bits(&mut self) -> bool {
            false
        }
    }

    impl RenderSliver for RenderSliverFixed {
        fn layout(&mut self, constraints: SliverConstraints) -> SliverGeometry {
            let remaining = constraints.remaining_paint_extent.get();
            let visible = (self.extent - constraints.scroll_offset.get()).clamp(0.0, remaining);
            SliverGeometry::new(self.extent, visible)
        }

        fn hit_test(
            &self,
            _: &mut HitTestResult,
            _: PositiveFinite<f32>,
            _: PositiveFinite<f32>,
        ) -> HitTest {
            HitTest::Pass
        }

        fn paint(&mut self, _: &mut PaintCtx, _: Offset) {}
    }

    #[test]
    fn viewport_drives_a_single_sliver() {
        let mut viewport =
            RenderViewport::new(RenderNode::new(RenderSliverFixed { extent: 100.0 }));

        let layout = LayoutPipeline::default();
        let mut paint = PaintPipeline::default();

        // 100 wide x 50 tall viewport: the sliver is 100 long, only 50 fits.
        let size = RenderBox::layout(
            &mut viewport,
            &mut LayoutCtx::new(&layout, &mut paint, LayoutScope::detached()),
            BoxConstraints::tight(Size::new(100.0, 50.0)),
        );
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
        RenderBox::layout(
            &mut viewport,
            &mut LayoutCtx::new(&layout, &mut paint, LayoutScope::detached()),
            BoxConstraints::tight(Size::new(100.0, 50.0)),
        );
        assert_eq!(viewport.geometry().unwrap().paint_extent.get(), 20.0);
    }

    /// A widget whose render object is a sliver — to exercise the erased boundary.
    struct SliverFixedWidget {
        extent: f32,
    }

    struct SliverFixedElement;

    impl Element for SliverFixedElement {
        type Render = RenderSliverFixed;
    }

    impl Widget for SliverFixedWidget {
        type Element = SliverFixedElement;

        type Render = RenderSliverFixed;

        fn create(self, _: &mut UpdateCtx) -> (SliverFixedElement, Self::Render) {
            (
                SliverFixedElement,
                RenderSliverFixed {
                    extent: self.extent,
                },
            )
        }

        fn update(self, _: &mut SliverFixedElement, object: &mut Self::Render, _: &mut UpdateCtx) {
            object.extent = self.extent;
        }
    }

    #[test]
    fn viewport_drives_an_erased_sliver() {
        // into_boxed_render_sliver erases to Box<dyn AnyRenderSliver>, which is itself a RenderSliver,
        // so the viewport drives it identically to a concrete sliver.
        let boxed_widget = SliverFixedWidget { extent: 100.0 }.into_boxed_render_sliver();
        let (_element, erased) = TestCtx::new().create(boxed_widget);

        let mut viewport = RenderViewport::new(RenderNode::new(erased));

        let layout = LayoutPipeline::default();
        let mut paint = PaintPipeline::default();

        RenderBox::layout(
            &mut viewport,
            &mut LayoutCtx::new(&layout, &mut paint, LayoutScope::detached()),
            BoxConstraints::tight(Size::new(100.0, 50.0)),
        );

        assert_eq!(viewport.geometry().unwrap().paint_extent.get(), 50.0);
    }
}

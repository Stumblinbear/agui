//! Sliver viewport tests from before the tree migration. Disabled until slivers are ported.
#![cfg(any())]
#![allow(clippy::float_cmp)]

use agui::{
    context::UpdateCtx,
    element::Element,
    geometry::{Offset, Size},
    input::hit_test::HitTestResult,
    prelude::render_object::LayoutScope,
    render_object::{
        LayoutCtx, RenderObject,
        box_layout::{BoxConstraints, RenderBox},
        node::RenderNode,
        sliver::{HitTest, RenderSliver, RenderViewport, SliverConstraints, SliverGeometry},
    },
    widget::{AsAnyWidget, Widget},
};
use agui_test::test_harness::TestCtx;
use typed_floats::PositiveFinite;

/// A sliver that occupies a fixed scroll extent and paints whatever of it is currently in widget.
struct RenderSliverFixed {
    extent: f32,
}

impl RenderObject for RenderSliverFixed {
    fn mount(&mut self, _: &mut MountCtx) {}

    fn unmount(&mut self, _: &mut MountCtx) {}
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

    fn paint(&mut self, _: &mut agui::context::PaintCtx, _: Offset) {}
}

#[test]
fn viewport_drives_a_single_sliver() {
    let mut viewport = RenderViewport::new(RenderNode::new(RenderSliverFixed { extent: 100.0 }));

    let layout = agui::pipeline::layout::LayoutPipeline::default();
    let mut paint = agui::pipeline::paint::PaintPipeline::default();

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

/// A widget whose render object is a sliver, to exercise the erased boundary.
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

    let layout = agui::pipeline::layout::LayoutPipeline::default();
    let mut paint = agui::pipeline::paint::PaintPipeline::default();

    RenderBox::layout(
        &mut viewport,
        &mut LayoutCtx::new(&layout, &mut paint, LayoutScope::detached()),
        BoxConstraints::tight(Size::new(100.0, 50.0)),
    );

    assert_eq!(viewport.geometry().unwrap().paint_extent.get(), 50.0);
}

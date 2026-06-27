use std::time::Duration;

use agui::{
    geometry::Size,
    paint::compositing::{CompositedFrame, CompositedNode, ExternalSurfaceId},
    render_object::box_layout::BoxConstraints,
    widgets::external_surface::ExternalSurface,
};
use agui_test::{ElementLifecycleCheck, WidgetTester, sizing::BoxSizingCheck};

fn external_node(frame: &CompositedFrame) -> (ExternalSurfaceId, Size) {
    frame
        .nodes()
        .iter()
        .find_map(|node| match node {
            CompositedNode::External { surface, size, .. } => Some((*surface, *size)),
            _ => None,
        })
        .expect("an external surface node")
}

#[test]
fn external_surface_obeys_the_element_lifecycle() {
    ElementLifecycleCheck::new().leaf(|| ExternalSurface::new(ExternalSurfaceId(7)));
}

#[test]
fn external_surface_obeys_box_sizing() {
    BoxSizingCheck::default().leaf(|| {
        ExternalSurface::new(ExternalSurfaceId(7))
            .width(20)
            .height(10)
    });
}

#[test]
fn places_a_surface_filling_its_bounds() {
    let mut tester = WidgetTester::mount(ExternalSurface::new(ExternalSurfaceId(42)));
    tester.resize_with(BoxConstraints::new(0, 64, 0, 48));
    tester.pump(Duration::ZERO);

    let (surface, size) = external_node(&tester.composite_frame());
    assert_eq!(surface, ExternalSurfaceId(42));
    assert_eq!(size, Size::new(64, 48), "fills the constraints it is given");
}

#[test]
fn an_explicit_size_overrides_filling() {
    let mut tester = WidgetTester::mount(
        ExternalSurface::new(ExternalSurfaceId(1))
            .width(20)
            .height(10),
    );
    tester.resize_with(BoxConstraints::new(0, 100, 0, 100));
    tester.pump(Duration::ZERO);

    let (_, size) = external_node(&tester.composite_frame());
    assert_eq!(size, Size::new(20, 10), "takes its explicit size");
}

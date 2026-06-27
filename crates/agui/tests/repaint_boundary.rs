use std::{rc::Rc, time::Duration};

use agui::{
    geometry::Size,
    paint::{command::PaintCommand, scene::Scene},
    widgets::repaint_boundary::RepaintBoundary,
};
use agui_test::{
    ElementLifecycleCheck, WidgetTester, fixtures::RecordingBox, sizing::BoxSizingCheck,
};

fn fills(scene: &Scene) -> usize {
    scene
        .commands()
        .iter()
        .filter(|command| matches!(command, PaintCommand::Fill { .. }))
        .count()
}

#[test]
fn repaint_boundary_obeys_the_element_lifecycle() {
    ElementLifecycleCheck::new().single_child(RepaintBoundary::new);
}

#[test]
fn repaint_boundary_obeys_box_sizing() {
    BoxSizingCheck::default().single_child(RepaintBoundary::new);
}

#[test]
fn a_marked_boundary_repaints_through_its_resolved_handle() {
    let render = RecordingBox::new();
    let paints = Rc::clone(&render.paints);
    let paint_boundary = Rc::clone(&render.paint_boundary);

    let mut tester = WidgetTester::mount(RepaintBoundary::new(render));
    tester.resize(Size::new(40, 40));
    tester.pump(Duration::ZERO);

    let first = tester.composite_frame().rasterize();
    assert_eq!(paints.get(), 1);
    assert_eq!(
        fills(&first),
        1,
        "the boundary's retained layer carries the fill"
    );

    // Mark the boundary through the scope its child captured. The deferred mark drains in `flush_layout`, so a
    // pump (which runs layout then paint) drains it and repaints the boundary's subtree through its handle.
    paint_boundary
        .borrow()
        .clone()
        .expect("the boundary is mounted")
        .mark_needs_paint();
    tester.pump(Duration::ZERO);

    let second = tester.composite_frame().rasterize();
    assert_eq!(
        paints.get(),
        2,
        "the marked boundary repainted through its handle"
    );
    assert_eq!(fills(&second), 1);
}

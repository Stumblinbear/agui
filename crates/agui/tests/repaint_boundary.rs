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

#[test]
fn a_boundary_whose_subtree_is_re_laid_repaints() {
    let render = RecordingBox::new();
    let paints = Rc::clone(&render.paints);
    let laid_out = Rc::clone(&render.laid_out);

    let mut tester = WidgetTester::mount(RepaintBoundary::new(render));
    tester.resize(Size::new(40, 40));
    tester.pump(Duration::ZERO);

    assert_eq!(paints.get(), 1);
    assert_eq!(laid_out.get(), Some(Size::new(40, 40)));

    // The root re-lays under the new constraints and recomputes the boundary's subtree, so the boundary's
    // retained layer no longer matches its content and must repaint. No layout boundary inside the repaint
    // boundary is marked; the re-lay alone changes the geometry.
    tester.resize(Size::new(80, 80));
    tester.pump(Duration::ZERO);

    assert_eq!(
        laid_out.get(),
        Some(Size::new(80, 80)),
        "the subtree was re-laid to the new size"
    );
    assert_eq!(
        paints.get(),
        2,
        "the re-laid boundary repainted its stale layer"
    );
}

#[test]
fn a_clean_subtree_under_unchanged_constraints_is_not_re_laid() {
    let render = RecordingBox::new();
    let layouts = Rc::clone(&render.layouts);
    let paints = Rc::clone(&render.paints);

    let mut tester = WidgetTester::mount(RepaintBoundary::new(render));
    tester.resize(Size::new(40, 40));
    tester.pump(Duration::ZERO);

    assert_eq!(layouts.get(), 1);
    assert_eq!(paints.get(), 1);

    // A same-size resize re-lays the root, whose walk reaches the child's relayout boundary with the same
    // tight constraints and nothing marked inside it: the subtree is skipped, not recomputed, and its
    // untouched layer is not repainted.
    tester.resize(Size::new(40, 40));
    tester.pump(Duration::ZERO);

    assert_eq!(
        layouts.get(),
        1,
        "unchanged constraints skip the clean subtree"
    );
    assert_eq!(paints.get(), 1, "a skipped subtree does not repaint");
}

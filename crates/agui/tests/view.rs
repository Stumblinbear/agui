use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use agui::{
    geometry::{Offset, Size},
    pipeline::PipelineOwner,
    semantics::{Role, Semantics},
    view::{View, ViewContainer},
};
use agui_test::{Probe, WidgetTester, fixtures::RecordingBox, test_harness::TestCtx};

#[test]
fn resize_lays_out_and_paints_the_child() {
    let probe = Probe::new();
    let mut tester = WidgetTester::mount(probe.wrap(()));

    tester.resize(Size::new(120, 80));
    tester.pump(Duration::ZERO);

    assert_eq!(
        probe.size(),
        Size::new(120, 80),
        "the view drove its child's layout"
    );
    assert!(probe.paints() >= 1, "the view drove its child's paint");

    // Compositing after a frame produces a presentable frame without panicking.
    let _frame = tester.composite_frame();
}

#[test]
fn resizing_again_relays_the_child() {
    let probe = Probe::new();
    let mut tester = WidgetTester::mount(probe.wrap(()));

    tester.resize(Size::new(100, 100));
    tester.pump(Duration::ZERO);
    assert_eq!(probe.size(), Size::new(100, 100));

    tester.resize(Size::new(50, 200));
    tester.pump(Duration::ZERO);
    assert_eq!(
        probe.size(),
        Size::new(50, 200),
        "the view re-laid its child under the new constraints"
    );
}

#[test]
fn resizing_repaints_through_the_layout_coupling() {
    let probe = Probe::new();
    let mut tester = WidgetTester::mount(probe.wrap(()));

    tester.resize(Size::new(100, 100));
    tester.pump(Duration::ZERO);
    let after_first = probe.paints();
    assert!(after_first >= 1, "the first frame painted the child");

    tester.resize(Size::new(50, 200));
    tester.pump(Duration::ZERO);
    assert!(
        probe.paints() > after_first,
        "re-laying the view repainted it via the layout-to-paint coupling"
    );
}

#[test]
fn hit_testing_an_empty_view_is_harmless() {
    // The `()` leaf passes hits through, so nothing absorbs. The call must still resolve to an empty path
    // rather than panic.
    let mut tester = WidgetTester::mount(());

    tester.resize(Size::new(64, 64));
    tester.pump(Duration::ZERO);

    assert!(tester.hit_test(Offset::new(10, 10)).path().is_empty());
}

#[test]
fn marking_a_child_boundary_relays_only_it() {
    let render = RecordingBox::new();
    let layouts = Rc::clone(&render.layouts);
    let boundary = Rc::clone(&render.boundary);

    let mut tester = WidgetTester::mount(render);

    // Tight constraints make the child its own relayout boundary, so marking it re-lays it without resizing
    // the view above.
    tester.resize(Size::new(40, 40));
    tester.pump(Duration::ZERO);
    assert_eq!(layouts.get(), 1, "the first frame laid the child out once");

    boundary
        .borrow()
        .as_ref()
        .expect("the child captured its boundary during layout")
        .mark_needs_layout();
    tester.pump(Duration::ZERO);
    assert_eq!(
        layouts.get(),
        2,
        "marking the child's own boundary re-laid it in isolation"
    );
}

#[test]
fn marking_one_view_rewalks_only_its_semantics() {
    let rb_a = RecordingBox::new();
    let a = Rc::clone(&rb_a.semantics_builds);
    let boundary_a = Rc::clone(&rb_a.semantics_boundary);
    let rb_b = RecordingBox::new();
    let b = Rc::clone(&rb_b.semantics_builds);

    let view_a = View::new(Rc::new(RefCell::new(None)))
        .child(Semantics::new().role(Role::Button).label("A").child(rb_a));
    let view_b = View::new(Rc::new(RefCell::new(None)))
        .child(Semantics::new().role(Role::Button).label("B").child(rb_b));

    // Mount the container as the owner root so the two views are siblings, the way a multi-window app hosts
    // them, rather than nested under a third view.
    let mut ctx = TestCtx::new();
    let owner = PipelineOwner::new(ViewContainer::new((view_a, view_b)), &mut ctx.scheduler());

    owner.flush_semantics();
    let (a_walked, b_walked) = (a.get(), b.get());

    // Mark only view a's boundary from outside a build pass, then re-walk. The flush applies the deferred
    // mark itself, and view b's boundary stays clean.
    boundary_a
        .borrow()
        .as_ref()
        .expect("the child captured its view's semantics boundary at attach")
        .mark_needs_semantics_update();
    owner.flush_semantics();

    assert!(a.get() > a_walked, "the marked view re-walked");
    assert_eq!(b.get(), b_walked, "the other view was not re-walked");
}

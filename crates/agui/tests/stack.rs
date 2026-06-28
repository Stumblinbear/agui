use std::time::Duration;

use agui::{
    geometry::{Alignment, Offset, Size},
    render_object::box_layout::BoxConstraints,
    widgets::stack::{Positioned, Stack, StackFit},
};
use agui_test::{
    Probe, WidgetTester,
    fixtures::{IntrinsicBox, TestBox},
};

#[test]
fn non_positioned_children_stack_at_the_alignment() {
    let back = Probe::new();
    let front = Probe::new();

    let stack = Stack::builder()
        .children((
            back.wrap(TestBox::new(Size::new(100, 40))),
            front.wrap(TestBox::new(Size::new(60, 80))),
        ))
        .build();

    let mut tester = WidgetTester::mount(stack);
    tester.resize_with(BoxConstraints::loose(Size::new(500, 500)));
    tester.pump(Duration::ZERO);

    // The default alignment is the top-left, so both layer at the origin.
    assert_eq!(back.offset(), Offset::new(0, 0));
    assert_eq!(front.offset(), Offset::new(0, 0));
    assert_eq!(back.size(), Size::new(100, 40));
    assert_eq!(front.size(), Size::new(60, 80));
}

#[test]
fn a_non_positioned_child_centers_under_a_center_alignment() {
    let child = Probe::new();

    let stack = Stack::builder()
        .alignment(Alignment::CENTER)
        .children((child.wrap(TestBox::new(Size::new(40, 40))),))
        .build();

    let mut tester = WidgetTester::mount(stack);
    tester.resize(Size::new(200, 200));
    tester.pump(Duration::ZERO);

    // The 40x40 child sits in the middle of the 200x200 stack: (200 - 40) / 2 on each axis.
    assert_eq!(child.offset(), Offset::new(80, 80));
}

#[test]
fn a_positioned_child_is_pinned_to_its_edges() {
    let pinned = Probe::new();

    let stack = Stack::builder()
        .children((
            // A non-positioned child sizes the stack to 200x200.
            TestBox::new(Size::new(200, 200)),
            Positioned::builder()
                .top(10.0)
                .left(20.0)
                .child(pinned.wrap(TestBox::new(Size::new(50, 30)))),
        ))
        .build();

    let mut tester = WidgetTester::mount(stack);
    tester.resize_with(BoxConstraints::loose(Size::new(500, 500)));
    tester.pump(Duration::ZERO);

    assert_eq!(pinned.offset(), Offset::new(20, 10));
    assert_eq!(pinned.size(), Size::new(50, 30));
}

#[test]
fn left_and_right_pin_a_positioned_child_to_a_tight_width() {
    let pinned = Probe::new();

    let stack = Stack::builder()
        .children((
            TestBox::new(Size::new(200, 200)),
            Positioned::builder()
                .left(10.0)
                .right(30.0)
                .top(0.0)
                .child(pinned.wrap(IntrinsicBox::new(Size::new(0, 50)))),
        ))
        .build();

    let mut tester = WidgetTester::mount(stack);
    tester.resize_with(BoxConstraints::loose(Size::new(500, 500)));
    tester.pump(Duration::ZERO);

    // Width is pinned to 200 - 10 - 30 = 160; height stays the child's own 50.
    assert_eq!(pinned.offset(), Offset::new(10, 0));
    assert_eq!(pinned.size(), Size::new(160, 50));
}

#[test]
fn expand_fit_grows_a_non_positioned_child_to_the_stack() {
    let child = Probe::new();

    let stack = Stack::builder()
        .fit(StackFit::Expand)
        .children((child.wrap(IntrinsicBox::new(Size::new(40, 40))),))
        .build();

    let mut tester = WidgetTester::mount(stack);
    tester.resize(Size::new(200, 150));
    tester.pump(Duration::ZERO);

    assert_eq!(child.offset(), Offset::new(0, 0));
    assert_eq!(child.size(), Size::new(200, 150));
}

#[test]
fn a_runtime_position_change_moves_a_tightly_sized_positioned_child() {
    let pinned = Probe::new();

    let build = |left: f32, pinned: &Probe| {
        Stack::builder()
            .children((
                TestBox::new(Size::new(200, 200)),
                Positioned::builder()
                    .left(left)
                    .top(10.0)
                    .width(50.0)
                    .height(30.0)
                    .child(pinned.wrap(IntrinsicBox::new(Size::new(50, 30)))),
            ))
            .build()
    };

    let mut tester = WidgetTester::mount(build(20.0, &pinned));
    tester.resize_with(BoxConstraints::loose(Size::new(500, 500)));
    tester.pump(Duration::ZERO);

    assert_eq!(pinned.offset(), Offset::new(20, 10));

    // width/height pin the child to a tight 50x30, so it is its own relayout boundary. The stack still
    // deposits its scope in the child's slot, so moving left 20 -> 60 at runtime re-lays the stack.
    tester.rebuild(build(60.0, &pinned));

    assert_eq!(pinned.offset(), Offset::new(60, 10));
    assert_eq!(pinned.size(), Size::new(50, 30));
}

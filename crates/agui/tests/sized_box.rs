use std::time::Duration;

use agui::{
    geometry::Size, render_object::box_layout::BoxConstraints, widgets::sized_box::SizedBox,
};
use agui_test::{ElementLifecycleCheck, Probe, WidgetTester, sizing::BoxSizingCheck};

#[test]
fn sized_box_obeys_the_element_lifecycle() {
    ElementLifecycleCheck::new().single_child(|child| SizedBox::new().child(child));
}

#[test]
fn sized_box_obeys_box_sizing() {
    BoxSizingCheck::default().leaf(|| SizedBox::new().width(40).height(30));
}

#[test]
fn uses_the_given_sizes() {
    let probe = Probe::new();
    // The `()` leaf takes the smallest size its constraints allow, and the `SizedBox` constrains it tightly,
    // so the probe reads back the size the box imposed.
    let mut tester =
        WidgetTester::mount(SizedBox::new().width(16).height(48).child(probe.wrap(())));
    tester.resize_with(BoxConstraints::new(0, 128, 0, 128));
    tester.pump(Duration::ZERO);

    assert_eq!(probe.size(), Size::new(16, 48));
}

#[test]
fn clamps_the_given_sizes_to_the_constraints() {
    let probe = Probe::new();
    let mut tester = WidgetTester::mount(SizedBox::new().width(0).height(16).child(probe.wrap(())));
    tester.resize_with(BoxConstraints::new(16, 128, 32, 128));
    tester.pump(Duration::ZERO);

    assert_eq!(probe.size(), Size::new(16, 32));
}

#[test]
fn expands_to_the_largest_size_within_the_constraints() {
    let probe = Probe::new();
    let mut tester = WidgetTester::mount(SizedBox::expand().child(probe.wrap(())));
    tester.resize_with(BoxConstraints::new(0, 128, 0, 128));
    tester.pump(Duration::ZERO);

    assert_eq!(probe.size(), Size::new(128, 128));
}

#[test]
fn a_childless_box_lays_out_and_paints() {
    // A childless `SizedBox` has a `()` child render object. Laying it out and painting it must run `()`'s
    // no-op `RenderBox` through its real (zero-sized) storage, not dereference an absent one.
    let mut tester = WidgetTester::mount(SizedBox::new().width(16).height(48));
    tester.resize_with(BoxConstraints::new(0, 128, 0, 128));
    tester.pump(Duration::ZERO);

    let _frame = tester.composite_frame();
}

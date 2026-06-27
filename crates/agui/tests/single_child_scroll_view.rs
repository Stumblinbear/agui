use std::time::Duration;

use agui::{
    geometry::Size, render_object::box_layout::BoxConstraints,
    widgets::single_child_scroll_view::SingleChildScrollView,
};
use agui_test::{ElementLifecycleCheck, Probe, WidgetTester, sizing::BoxSizingCheck};

#[test]
fn single_child_scroll_view_obeys_the_element_lifecycle() {
    ElementLifecycleCheck::new().single_child(SingleChildScrollView::new);
}

#[test]
fn single_child_scroll_view_obeys_box_sizing() {
    BoxSizingCheck::default().single_child(SingleChildScrollView::new);
}

#[test]
fn lays_its_child_out_with_the_width_constraints_and_an_unbounded_height() {
    let probe = Probe::new();
    // The `()` leaf takes the smallest size its constraints allow, so the probe reads back the size and
    // constraints the scroll view handed down.
    let mut tester = WidgetTester::mount(SingleChildScrollView::new(probe.wrap(())));
    tester.resize_with(BoxConstraints::new(16, 128, 32, 128));
    tester.pump(Duration::ZERO);

    let constraints = probe.constraints();
    assert_eq!(constraints.min_width().get(), 16.0);
    assert_eq!(constraints.max_width().get(), 128.0);
    assert_eq!(constraints.min_height().get(), 0.0);
    assert!(
        constraints.max_height().is_infinite(),
        "the scroll view leaves the child's height unbounded"
    );

    // The child takes the smallest size it can: its width is the minimum the scroll view passed through and
    // its height is 0, free to grow.
    assert_eq!(probe.size(), Size::new(16, 0));
}

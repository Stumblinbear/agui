use agui::geometry::Size;
use agui_test::{WidgetTester, fixtures::RecordingBox};

#[test]
fn a_layout_only_resize_leaves_the_tree_dirty() {
    let mut tester = WidgetTester::mount(RecordingBox::new());
    tester.resize(Size::new(40, 40));
    tester.pump_and_settle(8);

    assert!(
        !tester.is_dirty(),
        "the tree is settled after its first frame"
    );

    // A resize marks the root's relayout boundary but not the build queue. The tree must still report as
    // owed work so a driver polling it schedules a frame; a check that saw only the build queue would miss
    // the resize and leave it unapplied.
    tester.resize(Size::new(80, 80));

    assert!(tester.is_dirty(), "a layout-only resize is still owed work");
}

use std::{cell::Cell, rc::Rc, time::Duration};

use agui::{
    geometry::Size, render_object::box_layout::BoxConstraints, widget::AsAnyWidget,
    widgets::layout_builder::LayoutBuilder,
};
use agui_test::{WidgetTester, fixtures::TestBox};

// The builder runs during layout, once per constraint change, and the child it produces is mounted
// and laid out. Run under Miri, this exercises the layout-time mount: the render object reconciles
// its own child through a cursor while it is mid-layout-borrow.
#[test]
fn builds_its_child_during_layout_and_reruns_on_a_constraint_change() {
    let builds = Rc::new(Cell::new(0u32));

    let widget = LayoutBuilder::new({
        let builds = Rc::clone(&builds);
        move |constraints: BoxConstraints| {
            builds.set(builds.get() + 1);
            let size = if constraints.max_width().get() > 100.0 {
                80.0
            } else {
                20.0
            };
            TestBox::new(Size::new(size, size)).into_boxed_render_box()
        }
    });

    let mut tester = WidgetTester::mount(widget);

    tester.resize_with(BoxConstraints::new(0, 200, 0, 200));
    tester.pump(Duration::ZERO);
    assert_eq!(
        builds.get(),
        1,
        "the builder ran once during the first layout"
    );

    tester.resize_with(BoxConstraints::new(0, 50, 0, 50));
    tester.pump(Duration::ZERO);
    assert_eq!(
        builds.get(),
        2,
        "the builder reran for the changed constraints"
    );
}

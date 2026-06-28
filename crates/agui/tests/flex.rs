use std::time::Duration;

use agui::{
    geometry::{Offset, Size},
    key::Key,
    render_object::box_layout::BoxConstraints,
    widgets::flex::{Column, CrossAxisAlignment, Expanded, MainAxisAlignment, MainAxisSize, Row},
};
use agui_test::{
    Probe, WidgetTester,
    fixtures::{IntrinsicBox, TestBox},
};

#[test]
fn column_stacks_children_top_to_bottom() {
    let top = Probe::new();
    let bottom = Probe::new();

    let column = Column::builder()
        .main_axis_size(MainAxisSize::Min)
        .main_axis_alignment(MainAxisAlignment::Start)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .children((
            top.wrap(TestBox::new(Size::new(100, 20))),
            bottom.wrap(TestBox::new(Size::new(100, 30))),
        ))
        .build();

    let mut tester = WidgetTester::mount(column);
    tester.resize_with(BoxConstraints::loose(Size::new(500, 500)));
    tester.pump(Duration::ZERO);

    assert_eq!(top.size(), Size::new(100, 20));
    assert_eq!(bottom.size(), Size::new(100, 30));
    assert_eq!(top.offset(), Offset::new(0, 0));
    assert_eq!(bottom.offset(), Offset::new(0, 20));
}

#[test]
fn row_stacks_children_left_to_right() {
    let left = Probe::new();
    let right = Probe::new();

    let row = Row::builder()
        .main_axis_size(MainAxisSize::Min)
        .main_axis_alignment(MainAxisAlignment::Start)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .children((
            left.wrap(TestBox::new(Size::new(20, 100))),
            right.wrap(TestBox::new(Size::new(30, 100))),
        ))
        .build();

    let mut tester = WidgetTester::mount(row);
    tester.resize_with(BoxConstraints::loose(Size::new(500, 500)));
    tester.pump(Duration::ZERO);

    assert_eq!(left.offset(), Offset::new(0, 0));
    assert_eq!(right.offset(), Offset::new(20, 0));
}

#[test]
fn expanded_fills_the_space_the_inflexible_child_leaves() {
    let fixed = Probe::new();
    let flexible = Probe::new();

    let column = Column::builder()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .children((
            fixed.wrap(TestBox::new(Size::new(100, 20))),
            Expanded::builder().child(flexible.wrap(IntrinsicBox::new(Size::new(100, 0)))),
        ))
        .build();

    let mut tester = WidgetTester::mount(column);
    tester.resize(Size::new(200, 100));
    tester.pump(Duration::ZERO);

    // The fixed child keeps its 20, so the expanded child takes the remaining 80.
    assert_eq!(fixed.offset(), Offset::new(0, 0));
    assert_eq!(fixed.size(), Size::new(100, 20));
    assert_eq!(flexible.offset(), Offset::new(0, 20));
    assert_eq!(flexible.size(), Size::new(100, 80));
}

#[test]
fn two_expanded_children_split_the_space_by_flex_factor() {
    let one = Probe::new();
    let two = Probe::new();

    let column = Column::builder()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .children((
            Expanded::builder()
                .flex(1.0)
                .child(one.wrap(IntrinsicBox::new(Size::new(50, 0)))),
            Expanded::builder()
                .flex(3.0)
                .child(two.wrap(IntrinsicBox::new(Size::new(50, 0)))),
        ))
        .build();

    let mut tester = WidgetTester::mount(column);
    tester.resize(Size::new(200, 120));
    tester.pump(Duration::ZERO);

    // 120 split 1:3 is 30 and 90.
    assert_eq!(one.size(), Size::new(50, 30));
    assert_eq!(two.size(), Size::new(50, 90));
    assert_eq!(one.offset(), Offset::new(0, 0));
    assert_eq!(two.offset(), Offset::new(0, 30));
}

#[test]
fn space_between_pushes_children_to_the_ends() {
    let top = Probe::new();
    let bottom = Probe::new();

    let column = Column::builder()
        .main_axis_alignment(MainAxisAlignment::SpaceBetween)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .children((
            top.wrap(TestBox::new(Size::new(100, 20))),
            bottom.wrap(TestBox::new(Size::new(100, 20))),
        ))
        .build();

    let mut tester = WidgetTester::mount(column);
    tester.resize(Size::new(200, 100));
    tester.pump(Duration::ZERO);

    // 100 tall, 40 of content, so the 60 of slack lands entirely between the two.
    assert_eq!(top.offset(), Offset::new(0, 0));
    assert_eq!(bottom.offset(), Offset::new(0, 80));
}

#[test]
fn a_flex_child_keeps_its_flex_through_a_key() {
    let flexible = Probe::new();

    let column = Column::builder()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .children((
            TestBox::new(Size::new(100, 20)),
            Key::new(
                0_u32,
                Expanded::builder().child(flexible.wrap(IntrinsicBox::new(Size::new(100, 0)))),
            ),
        ))
        .build();

    let mut tester = WidgetTester::mount(column);
    tester.resize(Size::new(200, 100));
    tester.pump(Duration::ZERO);

    // The key's anchor forwards the expanded child's flex data, so it still takes the leftover 80.
    assert_eq!(flexible.offset(), Offset::new(0, 20));
    assert_eq!(flexible.size(), Size::new(100, 80));
}

#[test]
fn runtime_flex_change_redistributes_a_coupled_child() {
    let first = Probe::new();
    let second = Probe::new();

    let build = |second_flex: f32, first: &Probe, second: &Probe| {
        Column::builder()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .children((
                Expanded::builder()
                    .flex(1.0)
                    .child(first.wrap(IntrinsicBox::new(Size::new(50, 0)))),
                Expanded::builder()
                    .flex(second_flex)
                    .child(second.wrap(IntrinsicBox::new(Size::new(50, 0)))),
            ))
            .build()
    };

    let mut tester = WidgetTester::mount(build(1.0, &first, &second));
    tester.resize(Size::new(200, 120));
    tester.pump(Duration::ZERO);

    assert_eq!(first.size(), Size::new(50, 60));
    assert_eq!(second.size(), Size::new(50, 60));

    // Bump the second child's flex 1 -> 3 at runtime: the split should become 30 / 90. A Start cross axis
    // leaves the children coupled, so RenderFlexible captures the flex's scope and the mark lands right.
    tester.rebuild(build(3.0, &first, &second));

    assert_eq!(first.size(), Size::new(50, 30));
    assert_eq!(second.size(), Size::new(50, 90));
}

#[test]
fn runtime_flex_change_redistributes_a_stretched_expanded() {
    let first = Probe::new();
    let second = Probe::new();

    let build = |second_flex: f32, first: &Probe, second: &Probe| {
        Column::builder()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .children((
                Expanded::builder()
                    .flex(1.0)
                    .child(first.wrap(IntrinsicBox::new(Size::new(50, 0)))),
                Expanded::builder()
                    .flex(second_flex)
                    .child(second.wrap(IntrinsicBox::new(Size::new(50, 0)))),
            ))
            .build()
    };

    let mut tester = WidgetTester::mount(build(1.0, &first, &second));
    tester.resize(Size::new(200, 120));
    tester.pump(Duration::ZERO);

    assert_eq!(second.size(), Size::new(200, 60));

    // A Stretch cross axis makes each Expanded fully tight, so it is its own relayout boundary. The flex still
    // deposits its scope in each child's slot, so the runtime flex change re-lays the flex and the split
    // becomes 30 / 90 even though the child can't reach the flex on its own.
    tester.rebuild(build(3.0, &first, &second));

    assert_eq!(first.size(), Size::new(200, 30));
    assert_eq!(second.size(), Size::new(200, 90));
}

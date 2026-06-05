use bon::Builder;

use agui_core::{
    constraints::Constraints,
    context::{Dispatch, UpdateCtx},
    element::MultiChildElement,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    paint::PaintCtx,
    render_object::{LayoutScope, MountCtx, RenderObject, box_layout::RenderBox},
    routing_id::RoutingId,
    size::Size,
    text_baseline::TextBaseline,
    text_direction::TextDirection,
    widget::{AsAnyWidget, BoxedWidget, Widget},
};
use typed_floats::{Positive, PositiveFinite};

use crate::flex::{
    CrossAxisAlignment, Flexible, MainAxisAlignment, MainAxisSize, VerticalDirection,
};

#[derive(Builder)]
pub struct Column<Children> {
    #[builder(default)]
    main_axis_size: MainAxisSize,

    #[builder(default)]
    main_axis_alignment: MainAxisAlignment,

    #[builder(default)]
    cross_axis_alignment: CrossAxisAlignment,

    #[builder(default)]
    vertical_direction: VerticalDirection,

    text_direction: Option<TextDirection>,

    #[builder(with = FromIterator::from_iter)]
    children: Vec<Flexible<Children>>,
}

impl<Children, S: column_builder::State> ColumnBuilder<Children, S>
where
    Children: AsAnyWidget,
    Children::Render: RenderBox,
{
    #[allow(deprecated)]
    pub fn dyn_children(
        self,
        iter: impl IntoIterator<Item = Flexible<Children>>,
    ) -> ColumnBuilder<BoxedWidget, column_builder::SetChildren<S>>
    where
        S::Children: column_builder::IsUnset,
    {
        ColumnBuilder {
            __unsafe_private_phantom: ::core::marker::PhantomData,
            __unsafe_private_named: (
                self.__unsafe_private_named.0,
                self.__unsafe_private_named.1,
                self.__unsafe_private_named.2,
                self.__unsafe_private_named.3,
                self.__unsafe_private_named.4,
                Some(FromIterator::from_iter(iter.into_iter().map(|c| {
                    Flexible {
                        child: c.child.into_boxed_render_box(),

                        flex: c.flex,
                        fit: c.fit,
                    }
                }))),
            ),
        }
    }
}

impl<Children> Widget for Column<Children>
where
    Children: Widget,
    Children::Render: RenderObject,
{
    type Element = MultiChildElement<Children::Element>;

    type Render = RenderFlex<Children::Render>;

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        MultiChildElement::new(self.children.len(), |i| &self.children[i].child, ctx)
    }

    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
        element.update(
            self.children.len(),
            |i| &self.children[i].child,
            |i| &old.children[i].child,
            ctx,
        );
    }

    fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
        element.dispatch(|i| &self.children[i].child, path, action)
    }

    fn create_render_object(&self, element: &Self::Element) -> Self::Render {
        RenderFlex {
            main_axis_size: self.main_axis_size,
            main_axis_alignment: self.main_axis_alignment,
            cross_axis_alignment: self.cross_axis_alignment,
            vertical_direction: self.vertical_direction,
            text_direction: self.text_direction,

            children: self
                .children
                .iter()
                .enumerate()
                .map(|(idx, flexible)| {
                    flexible
                        .child
                        .create_render_object(&element.children[idx].element)
                })
                .collect(),

            size: Size::ZERO,
        }
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        if render_object.main_axis_size != self.main_axis_size {
            render_object.main_axis_size = self.main_axis_size;
        }

        if render_object.main_axis_alignment != self.main_axis_alignment {
            render_object.main_axis_alignment = self.main_axis_alignment;
        }

        if render_object.cross_axis_alignment != self.cross_axis_alignment {
            render_object.cross_axis_alignment = self.cross_axis_alignment;
        }

        if render_object.vertical_direction != self.vertical_direction {
            render_object.vertical_direction = self.vertical_direction;
        }

        if render_object.text_direction != self.text_direction {
            render_object.text_direction = self.text_direction;
        }

        for (idx, (child, flexible)) in render_object
            .children
            .iter_mut()
            .zip(self.children.iter())
            .enumerate()
        {
            flexible
                .child
                .update_render_object(&element.children[idx].element, child);
        }
    }
}

pub struct RenderFlex<Children> {
    main_axis_size: MainAxisSize,
    main_axis_alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
    vertical_direction: VerticalDirection,
    text_direction: Option<TextDirection>,

    children: Vec<Children>,

    size: Size,
}

impl<Child> RenderObject for RenderFlex<Child>
where
    Child: RenderObject,
{
    fn mount(&mut self, ctx: &mut MountCtx) {
        for child in &mut self.children {
            child.mount(ctx);
        }
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        for child in &mut self.children {
            child.unmount(ctx);
        }
    }

    fn update_compositing_bits(&mut self) -> bool {
        let mut needs = false;

        for child in &mut self.children {
            needs |= child.update_compositing_bits();
        }

        needs
    }
}

impl<Child> RenderBox for RenderFlex<Child>
where
    Child: RenderBox,
{
    fn min_intrinsic_width(&self, _height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_width(&self, _height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn min_intrinsic_height(&self, _width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_height(&self, _width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn measure(&self, _constraints: Constraints) -> Size {
        Size::ZERO
    }

    fn layout(&mut self, _: &LayoutScope, _: Constraints) -> Size {
        Size::ZERO
    }

    fn measure_baseline(
        &self,
        _constraints: Constraints,
        _baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, _result: &mut HitTestResult, position: Offset) -> HitTest {
        if !self.size.contains(position) {
            return HitTest::Pass;
        }

        HitTest::Pass
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _offset: Offset) {}
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agui_core::{
        element::Element, key::Key, render_object::RenderLeaf, test_harness::TestHarness,
        widget::AsAnyWidget,
    };

    use super::*;

    thread_local! {
        static MOUNT_COUNT: RefCell<usize> = const { RefCell::new(0) };
        static UPDATE_COUNT: RefCell<usize> = const { RefCell::new(0) };
    }

    pub struct TestWidget<T> {
        value: T,
    }

    impl<T> TestWidget<T> {
        pub fn new(value: T) -> Self {
            Self { value }
        }
    }

    pub struct TestWidgetElement<T> {
        value: T,
    }

    impl<T: 'static> Element for TestWidgetElement<T> {}

    impl<T> Widget for TestWidget<T>
    where
        T: Clone + 'static,
    {
        type Element = TestWidgetElement<T>;

        type Render = RenderLeaf;

        fn create_element(&self, _: &mut UpdateCtx) -> Self::Element {
            MOUNT_COUNT.with(|count| *count.borrow_mut() += 1);

            TestWidgetElement {
                value: self.value.clone(),
            }
        }

        fn update(&self, element: &mut Self::Element, _: &Self, _: &mut UpdateCtx) {
            UPDATE_COUNT.with(|count| *count.borrow_mut() += 1);

            element.value = self.value.clone();
        }

        fn create_render_object(&self, _: &Self::Element) -> RenderLeaf {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Self::Element, _: &mut RenderLeaf) {}
    }

    #[test]
    fn column_builder() {
        let _ = Column::builder()
            .children([TestWidget::new(0).into(), TestWidget::new(1).into()])
            .build();

        let _ = Column::builder()
            .children(bon::vec![
                TestWidget::new(0),
                TestWidget::new(0),
                Flexible::from(TestWidget::new(0)),
            ])
            .build();

        let _ = Column::builder()
            .children(bon::vec![
                TestWidget::new(0).into_boxed_render_box(),
                TestWidget::new(0).into_boxed_render_box(),
                Flexible::from(TestWidget::new(0).into_boxed_render_box()),
            ])
            .build();

        let _ = Column::builder()
            .dyn_children(bon::vec![
                TestWidget::new(0),
                TestWidget::new(0),
                Flexible::from(TestWidget::new(0)),
            ])
            .build();
    }

    #[test]
    fn adds_all_children() {
        let column = Column::builder()
            .children([
                TestWidget::new(0).into(),
                TestWidget::new(0).into(),
                TestWidget::new(0).into(),
            ])
            .build();

        let harness = TestHarness::mount(&column);

        assert_eq!(harness.root.element.children.len(), 3);
    }

    #[test]
    fn only_remounts_children_when_children_replaced() {
        let column_1 = Column::builder()
            .children([
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
            ])
            .build();

        let mut harness = TestHarness::mount(&column_1);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 2);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);

        let column_2 = Column::builder()
            .children([
                TestWidget::<u16>::new(0).into_boxed_render_box().into(),
                TestWidget::<u16>::new(0).into_boxed_render_box().into(),
            ])
            .build();

        harness.update(&column_1, &column_2);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 4);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);
    }

    #[test]
    fn only_updates_children_when_children_unchanged() {
        let column_1 = Column::builder()
            .children([
                TestWidget::<usize>::new(0).into(),
                TestWidget::<usize>::new(0).into(),
            ])
            .build();

        let mut harness = TestHarness::mount(&column_1);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 2);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);

        let column_2 = Column::builder()
            .children([
                TestWidget::<usize>::new(0).into(),
                TestWidget::<usize>::new(0).into(),
            ])
            .build();

        harness.update(&column_1, &column_2);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 2);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 2);
    }

    #[test]
    fn retains_leading_unchanged_children() {
        let column_1 = Column::builder()
            .children([
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
            ])
            .build();

        let mut harness = TestHarness::mount(&column_1);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 5);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);

        let column_2 = Column::builder()
            .children([
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<u32>::new(0).into_boxed_render_box().into(),
            ])
            .build();

        harness.update(&column_1, &column_2);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 6);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 2);
    }

    #[test]
    fn retains_following_unchanged_children() {
        let column_1 = Column::builder()
            .children([
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
            ])
            .build();

        let mut harness = TestHarness::mount(&column_1);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 5);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);

        let column_2 = Column::builder()
            .children([
                TestWidget::<u32>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
            ])
            .build();

        harness.update(&column_1, &column_2);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 6);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 2);
    }

    #[test]
    fn retains_leading_and_following_unchanged_children() {
        let column_1 = Column::builder()
            .children([
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
            ])
            .build();

        let mut harness = TestHarness::mount(&column_1);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 5);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);

        let column_2 = Column::builder()
            .children([
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<u32>::new(0).into_boxed_render_box().into(),
                TestWidget::<u32>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
            ])
            .build();

        harness.update(&column_1, &column_2);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 7);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 4);
    }

    #[test]
    fn retains_middle_keyed_child() {
        let column_1 = Column::builder()
            .children([
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                Key::new(0, TestWidget::<usize>::new(0))
                    .into_boxed_render_box()
                    .into(),
                Key::new(1, TestWidget::<usize>::new(0))
                    .into_boxed_render_box()
                    .into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
                TestWidget::<usize>::new(0).into_boxed_render_box().into(),
            ])
            .build();

        let mut harness = TestHarness::mount(&column_1);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 6);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);

        let column_2 = Column::builder()
            .children([
                TestWidget::<u32>::new(0).into_boxed_render_box().into(),
                Key::new(0, TestWidget::<usize>::new(0))
                    .into_boxed_render_box()
                    .into(),
                Key::new(2, TestWidget::<usize>::new(0))
                    .into_boxed_render_box()
                    .into(),
                TestWidget::<u32>::new(0).into_boxed_render_box().into(),
            ])
            .build();

        harness.update(&column_1, &column_2);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 9);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 1);
    }
}

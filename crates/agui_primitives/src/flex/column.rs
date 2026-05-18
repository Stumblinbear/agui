use bon::Builder;

use agui_core::{
    constraints::Constraints,
    context::{Dispatch, UpdateCtx},
    element::Element,
    hit_test::{HitTest, HitTestResult},
    key::AnyKeyable,
    offset::Offset,
    render_object::{
        AsAnyRenderObject, RenderObject,
        box_layout::{BoxLayout, RenderBox},
    },
    renderer::Canvas,
    routing_id::RoutingId,
    size::Size,
    text_baseline::TextBaseline,
    text_direction::TextDirection,
    view::{AsAnyView, BoxedView, View},
};
use fnv::FnvHashMap;
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
    Children: AsAnyView,
    Children::Render: AsAnyRenderObject,
{
    #[allow(deprecated)]
    pub fn dyn_children(
        self,
        iter: impl IntoIterator<Item = Flexible<Children>>,
    ) -> ColumnBuilder<BoxedView<Children>, column_builder::SetChildren<S>>
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
                        child: c.child.into_boxed_view(),

                        flex: c.flex,
                        fit: c.fit,
                    }
                }))),
            ),
        }
    }
}

impl<Children> View for Column<Children>
where
    Children: View,
    Children::Render: RenderObject,
{
    type Render = RenderFlex<Children::Render>;

    type State = ();

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        (
            self.children
                .iter()
                .enumerate()
                .map(|(idx, flexible)| {
                    ctx.with_routing_id(RoutingId::new(idx as u16), |ctx| {
                        Element::new(&flexible.child, ctx)
                    })
                })
                .collect(),
            (),
        )
    }

    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
        // If we no longer have any children, make sure we clear out the element's children
        if self.children.is_empty() {
            element.children.clear();

            return;
        }

        // If we had no children before, we can just insert all the new children
        if element.children.is_empty() {
            element.children = self
                .children
                .iter()
                .enumerate()
                .map(|(idx, flexible)| {
                    ctx.with_routing_id(RoutingId::new(idx as u16), |ctx| {
                        Element::new(&flexible.child, ctx)
                    })
                })
                .collect();

            return;
        }

        assert!(
            old.children.len() == element.children.len(),
            "column children count mismatch"
        );

        let span = tracing::trace_span!("children", child_id = tracing::field::Empty);
        let _enter = span.enter();

        let mut new_children_top = 0;
        let mut old_children_top = 0;
        let mut new_children_bottom = self.children.len() - 1;
        let mut old_children_bottom = old.children.len() - 1;

        let mut old_child_elements = (0..self.children.len())
            .map(|_| Element::empty())
            .collect::<Vec<_>>();

        std::mem::swap(&mut element.children, &mut old_child_elements);

        // Update the top of the list.
        while (old_children_top <= old_children_bottom) && (new_children_top <= new_children_bottom)
        {
            if tracing::span_enabled!(tracing::Level::TRACE) {
                span.record("child_id", old_children_top);
            }

            let old_child = old.children.get(old_children_top);
            let new_child = self.children.get(new_children_top);

            if let Some((old_child, new_child)) = old_child.zip(new_child) {
                if !old_child.child.is_same_type(&new_child.child) {
                    break;
                }

                std::mem::swap(
                    &mut element.children[new_children_top],
                    &mut old_child_elements[old_children_top],
                );

                ctx.with_routing_id(RoutingId::new(new_children_top as u16), |ctx| {
                    element
                        .child_mut(new_children_top, &old.children[old_children_top].child)
                        .update(&new_child.child, ctx)
                });
            } else {
                break;
            }

            new_children_top += 1;
            old_children_top += 1;
        }

        // Scan the bottom of the list.
        while (old_children_top <= old_children_bottom) && (new_children_top <= new_children_bottom)
        {
            if tracing::span_enabled!(tracing::Level::TRACE) {
                span.record("child_id", old_children_bottom);
            }

            let old_child = old.children.get(old_children_bottom);
            let new_child = self.children.get(new_children_bottom);

            if let Some((old_child, new_child)) = old_child.zip(new_child) {
                if !old_child.child.is_same_type(&new_child.child) {
                    break;
                }
            } else {
                break;
            }

            old_children_bottom -= 1;
            new_children_bottom -= 1;
        }

        // Scan the old children in the middle of the list.
        let have_old_children = old_children_top <= old_children_bottom;

        #[allow(clippy::mutable_key_type)]
        let mut old_keyed_children = FnvHashMap::<&dyn AnyKeyable, usize>::default();

        while old_children_top <= old_children_bottom {
            // TODO(trevin): does this need to do .get(old_children_top)?
            if let Some(old_child_key) = old.children[old_children_top].child.key() {
                old_keyed_children.insert(old_child_key, old_children_top);
            }

            old_children_top += 1;
        }

        let children_len = self.children.len();
        let mut children = self.children.iter().skip(new_children_top);

        let initial_top = new_children_top;

        // Update the middle of the list.
        while new_children_top <= new_children_bottom {
            if tracing::span_enabled!(tracing::Level::TRACE) {
                span.record("child_id", new_children_top);
            }

            let new_child = match children.next() {
                Some(new_child) => new_child,
                None => unreachable!(
                    "new children should never run out: {} {}-{}/{}",
                    initial_top, new_children_top, new_children_bottom, children_len
                ),
            };

            let mut existing_child_idx: Option<usize> = None;

            if have_old_children
                && let Some(old_child_idx) = new_child.child.key().and_then(|key| {
                    // Remove it from the list so that we don't try to use it again.
                    old_keyed_children.remove(&key)
                })
            {
                existing_child_idx = Some(old_child_idx);
            }

            if let Some(existing_child_idx) = existing_child_idx {
                std::mem::swap(
                    &mut element.children[new_children_top],
                    &mut old_child_elements[existing_child_idx],
                );

                ctx.with_routing_id(RoutingId::new(new_children_top as u16), |ctx| {
                    element
                        .child_mut(new_children_top, &old.children[existing_child_idx].child)
                        .update(&new_child.child, ctx)
                });
            } else {
                element.children[new_children_top] = ctx
                    .with_routing_id(RoutingId::new(new_children_top as u16), |ctx| {
                        Element::new(&new_child.child, ctx)
                    })
            }

            new_children_top += 1;
        }

        if tracing::span_enabled!(tracing::Level::TRACE) {
            span.record("child_id", tracing::field::Empty);
        }

        // We've scanned the whole list.
        assert_eq!(old_children_top, old_children_bottom + 1);
        assert_eq!(new_children_top, new_children_bottom + 1);
        assert_eq!(
            children_len - new_children_top,
            old.children.len() - old_children_top
        );

        new_children_bottom = children_len - 1;
        old_children_bottom = old.children.len() - 1;

        // Update the bottom of the list.
        while (old_children_top <= old_children_bottom) && (new_children_top <= new_children_bottom)
        {
            if tracing::span_enabled!(tracing::Level::TRACE) {
                span.record("child_id", new_children_top);
            }

            std::mem::swap(
                &mut element.children[new_children_top],
                &mut old_child_elements[old_children_top],
            );

            ctx.with_routing_id(RoutingId::new(new_children_top as u16), |ctx| {
                element
                    .child_mut(new_children_top, &old.children[old_children_top].child)
                    .update(&self.children[new_children_top].child, ctx)
            });

            new_children_top += 1;
            old_children_top += 1;
        }
    }

    fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
        let Some((head, rest)) = path.split_first() else {
            unreachable!("dispatch path cannot be empty");
        };

        let child_idx = head.get() as usize;

        element
            .child_mut(child_idx, &self.children[child_idx].child)
            .dispatch(rest, action)
    }

    fn create_render_object(&self, element: &Element) -> Self::Render {
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
                .map(|(idx, flexible)| element.child(idx, &flexible.child).create_render_object())
                .collect(),

            size: Size::ZERO,
        }
    }

    fn update_render_object(&self, element: &Element, render_object: &mut Self::Render) {
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
            element
                .child(idx, &flexible.child)
                .update_render_object(child);
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
    fn mount(&mut self, ctx: &mut UpdateCtx) {
        for child in &mut self.children {
            child.mount(ctx);
        }
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx) {
        for child in &mut self.children {
            child.unmount(ctx);
        }
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        if !self.size.contains(position) {
            return HitTest::Pass;
        }

        HitTest::Pass
    }

    fn draw(&mut self, canvas: &mut Canvas) {}
}

impl<Child> BoxLayout for RenderFlex<Child>
where
    Child: RenderBox,
{
    fn size(&self) -> Size {
        self.size
    }

    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn measure(&self, constraints: Constraints) -> Size {
        Size::ZERO
    }

    fn layout(&mut self, constraints: Constraints) {}

    fn measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }
}

impl<Child> AsAnyRenderObject for RenderFlex<Child>
where
    Self: RenderBox,
{
    type Output = dyn agui_core::render_object::box_layout::AnyRenderBox;

    fn as_dyn_render_object(&self) -> &dyn agui_core::render_object::AnyRenderObject {
        self
    }

    fn into_boxed_render_object(self) -> Box<Self::Output> {
        Box::new(self)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agui_core::{
        key::Key, render_object::RenderLeaf, test_harness::TestHarness, view::AsAnyView,
    };

    use super::*;

    thread_local! {
        static MOUNT_COUNT: RefCell<usize> = const { RefCell::new(0) };
        static UPDATE_COUNT: RefCell<usize> = const { RefCell::new(0) };
    }

    pub struct TestView<T> {
        value: T,
    }

    impl<T> TestView<T> {
        pub fn new(value: T) -> Self {
            Self { value }
        }
    }

    impl<T> View for TestView<T>
    where
        T: Clone + 'static,
    {
        type Render = RenderLeaf;

        type State = T;

        fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            MOUNT_COUNT.with(|count| *count.borrow_mut() += 1);

            (vec![], self.value.clone())
        }

        fn update(&self, element: &mut Element, _: &Self, _: &mut UpdateCtx) {
            UPDATE_COUNT.with(|count| *count.borrow_mut() += 1);

            *element.state.downcast_mut::<Self>() = self.value.clone();
        }

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    #[test]
    fn column_builder() {
        let _ = Column::builder()
            .children([TestView::new(0).into(), TestView::new(1).into()])
            .build();

        let _ = Column::builder()
            .children(bon::vec![
                TestView::new(0),
                TestView::new(0),
                Flexible::from(TestView::new(0)),
            ])
            .build();

        let _ = Column::builder()
            .children(bon::vec![
                TestView::new(0).into_boxed_view(),
                TestView::new(0).into_boxed_view(),
                Flexible::from(TestView::new(0).into_boxed_view()),
            ])
            .build();

        let _ = Column::builder()
            .dyn_children(bon::vec![
                TestView::new(0),
                TestView::new(0),
                Flexible::from(TestView::new(0)),
            ])
            .build();
    }

    #[test]
    fn adds_all_children() {
        let column = Column::builder()
            .children([
                TestView::new(0).into(),
                TestView::new(0).into(),
                TestView::new(0).into(),
            ])
            .build();

        let harness = TestHarness::mount(&column);

        assert_eq!(harness.root.children.len(), 3);
    }

    #[test]
    fn only_remounts_children_when_children_replaced() {
        let column_1 = Column::builder()
            .children([
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
            ])
            .build();

        let mut harness = TestHarness::mount(&column_1);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 2);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);

        let column_2 = Column::builder()
            .children([
                TestView::<u16>::new(0).into_boxed_view().into(),
                TestView::<u16>::new(0).into_boxed_view().into(),
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
                TestView::<usize>::new(0).into(),
                TestView::<usize>::new(0).into(),
            ])
            .build();

        let mut harness = TestHarness::mount(&column_1);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 2);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);

        let column_2 = Column::builder()
            .children([
                TestView::<usize>::new(0).into(),
                TestView::<usize>::new(0).into(),
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
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
            ])
            .build();

        let mut harness = TestHarness::mount(&column_1);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 5);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);

        let column_2 = Column::builder()
            .children([
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<u32>::new(0).into_boxed_view().into(),
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
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
            ])
            .build();

        let mut harness = TestHarness::mount(&column_1);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 5);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);

        let column_2 = Column::builder()
            .children([
                TestView::<u32>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
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
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
            ])
            .build();

        let mut harness = TestHarness::mount(&column_1);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 5);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);

        let column_2 = Column::builder()
            .children([
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<u32>::new(0).into_boxed_view().into(),
                TestView::<u32>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
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
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                Key::new(0, TestView::<usize>::new(0))
                    .into_boxed_view()
                    .into(),
                Key::new(1, TestView::<usize>::new(0))
                    .into_boxed_view()
                    .into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
                TestView::<usize>::new(0).into_boxed_view().into(),
            ])
            .build();

        let mut harness = TestHarness::mount(&column_1);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 6);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);

        let column_2 = Column::builder()
            .children([
                TestView::<u32>::new(0).into_boxed_view().into(),
                Key::new(0, TestView::<usize>::new(0))
                    .into_boxed_view()
                    .into(),
                Key::new(2, TestView::<usize>::new(0))
                    .into_boxed_view()
                    .into(),
                TestView::<u32>::new(0).into_boxed_view().into(),
            ])
            .build();

        harness.update(&column_1, &column_2);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 9);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 1);
    }
}

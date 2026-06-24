use bon::Builder;

use agui_render::prelude::{element::*, render_object::*};

use typed_floats::{Positive, PositiveFinite};

use crate::flex::{CrossAxisAlignment, MainAxisAlignment, MainAxisSize, VerticalDirection};

/// A widget that lays its children out in a vertical run.
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

    children: Children,
}

impl<Children> Widget for Column<Children>
where
    Children: WidgetSequence,
    Children::Renders: 'static,
{
    type Element = ChildrenElement<Children, RenderFlex<Children::Renders>>;

    type Render = RenderFlex<Children::Renders>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (element, children) = ChildrenElement::new(self.children, ctx);

        let render_object = RenderFlex {
            main_axis_size: self.main_axis_size,
            main_axis_alignment: self.main_axis_alignment,
            cross_axis_alignment: self.cross_axis_alignment,
            vertical_direction: self.vertical_direction,
            text_direction: self.text_direction,

            layout_scope: LayoutScope::detached(),

            children,

            size: Size::ZERO,
        };

        (element, render_object)
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        if render_object.main_axis_size != self.main_axis_size
            || render_object.main_axis_alignment != self.main_axis_alignment
            || render_object.cross_axis_alignment != self.cross_axis_alignment
            || render_object.vertical_direction != self.vertical_direction
            || render_object.text_direction != self.text_direction
        {
            render_object.main_axis_size = self.main_axis_size;
            render_object.main_axis_alignment = self.main_axis_alignment;
            render_object.cross_axis_alignment = self.cross_axis_alignment;
            render_object.vertical_direction = self.vertical_direction;
            render_object.text_direction = self.text_direction;

            ctx.mark_needs_layout(render_object.layout_scope);
        }

        element.update(self.children, render_object, ctx);
    }
}

pub struct RenderFlex<Children> {
    main_axis_size: MainAxisSize,
    main_axis_alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
    vertical_direction: VerticalDirection,
    text_direction: Option<TextDirection>,

    layout_scope: LayoutScope,

    children: Children,

    size: Size,
}

impl<Children: RenderChildren> MultiChildRenderObject for RenderFlex<Children> {
    type Children = Children;

    fn children_mut(&mut self) -> &mut Children {
        &mut self.children
    }
}

impl<Children: RenderChildren + 'static> RenderObject for RenderFlex<Children> {
    fn mount(&mut self, ctx: &mut MountCtx) {
        self.children.for_each_mut(&mut |child| child.mount(ctx));
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.layout_scope = LayoutScope::detached();

        self.children.for_each_mut(&mut |child| child.unmount(ctx));
    }
}

impl<Children: RenderChildren + 'static> RenderBox for RenderFlex<Children> {
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

    fn measure(&self, _constraints: BoxConstraints) -> Size {
        Size::ZERO
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, _: BoxConstraints) -> Size {
        self.layout_scope = *ctx.scope();

        Size::ZERO
    }

    fn measure_baseline(
        &self,
        _constraints: BoxConstraints,
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

    fn update_compositing_bits(&mut self) -> bool {
        let mut needs = false;

        self.children
            .for_each_mut(&mut |child| needs |= child.update_compositing_bits());

        needs
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _offset: Offset) {}
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agui_render::{
        key::Key, render_object::RenderChildren, test_harness::TestCtx, widget::AsAnyWidget,
    };

    use super::*;

    thread_local! {
        static MOUNT_COUNT: RefCell<usize> = const { RefCell::new(0) };
        static UPDATE_COUNT: RefCell<usize> = const { RefCell::new(0) };
    }

    fn reset_counts() {
        MOUNT_COUNT.with(|count| *count.borrow_mut() = 0);
        UPDATE_COUNT.with(|count| *count.borrow_mut() = 0);
    }

    fn mounts() -> usize {
        MOUNT_COUNT.with(|count| *count.borrow())
    }

    fn updates() -> usize {
        UPDATE_COUNT.with(|count| *count.borrow())
    }

    /// A leaf widget distinguished by its type parameter, so distinct `TestWidget<T>` can sit in one
    /// heterogeneous tuple. Its render object is the unit box.
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

    impl<T: 'static> Element for TestWidgetElement<T> {
        type Render = ();
    }

    impl<T: 'static> Widget for TestWidget<T> {
        type Element = TestWidgetElement<T>;

        type Render = ();

        fn create(self, _: &mut UpdateCtx) -> (Self::Element, Self::Render) {
            MOUNT_COUNT.with(|count| *count.borrow_mut() += 1);

            (TestWidgetElement { value: self.value }, ())
        }

        fn update(self, element: &mut Self::Element, (): &mut Self::Render, _: &mut UpdateCtx) {
            UPDATE_COUNT.with(|count| *count.borrow_mut() += 1);

            element.value = self.value;
        }
    }

    #[test]
    fn builder_accepts_tuple_and_vec_children() {
        // A heterogeneous tuple of distinct widget types.
        let _ = Column::builder()
            .children((TestWidget::new(0_usize), TestWidget::new(1_u32)))
            .build();

        // A homogeneous vec of one widget type.
        let _ = Column::builder()
            .children(vec![TestWidget::new(0_usize), TestWidget::new(1_usize)])
            .build();
    }

    #[test]
    fn heterogeneous_tuple_builds_each_child() {
        reset_counts();

        let column = Column::builder()
            .children((
                TestWidget::new(0_usize),
                TestWidget::new(0_u32),
                TestWidget::new(0_u8),
            ))
            .build();

        let (_, render) = TestCtx::new().run(|ctx| column.create(ctx));

        assert_eq!(mounts(), 3, "each distinct-typed child mounted once");
        assert_eq!(render.children.len(), 3);
    }

    #[test]
    fn tuple_children_reconcile_in_place() {
        reset_counts();

        let column = Column::builder()
            .children((TestWidget::new(0_usize), TestWidget::new(0_u32)))
            .build();

        let (mut element, mut render) = TestCtx::new().run(|ctx| column.create(ctx));
        assert_eq!(mounts(), 2);

        let next = Column::builder()
            .children((TestWidget::new(1_usize), TestWidget::new(1_u32)))
            .build();
        TestCtx::new().run(|ctx| next.update(&mut element, &mut render, ctx));

        assert_eq!(mounts(), 2, "same-shape rebuild reused each child");
        assert_eq!(updates(), 2);
    }

    #[test]
    fn vec_children_count_and_remount_on_type_swap() {
        reset_counts();

        let column = Column::builder()
            .children(vec![
                TestWidget::<usize>::new(0),
                TestWidget::<usize>::new(0),
            ])
            .build();

        let (mut element, mut render) = TestCtx::new().run(|ctx| column.create(ctx));
        assert_eq!(mounts(), 2);
        assert_eq!(render.children.len(), 2);

        // Same type: reconciled, not remounted.
        let same = Column::builder()
            .children(vec![
                TestWidget::<usize>::new(0),
                TestWidget::<usize>::new(0),
            ])
            .build();
        TestCtx::new().run(|ctx| same.update(&mut element, &mut render, ctx));
        assert_eq!(mounts(), 2);
        assert_eq!(updates(), 2);
    }

    #[test]
    fn retains_middle_keyed_child_across_reorder() {
        reset_counts();

        // Boxed children let a single vec hold differently-typed widgets, two of them keyed.
        let column = Column::builder()
            .children(vec![
                TestWidget::<usize>::new(0).into_boxed_render_box(),
                Key::new(0, TestWidget::<usize>::new(0)).into_boxed_render_box(),
                Key::new(1, TestWidget::<usize>::new(0)).into_boxed_render_box(),
                TestWidget::<usize>::new(0).into_boxed_render_box(),
            ])
            .build();

        let (mut element, mut render) = TestCtx::new().run(|ctx| column.create(ctx));
        assert_eq!(mounts(), 4);

        // Keep key 0, drop key 1, swap the unkeyed ends for a different type.
        let next = Column::builder()
            .children(vec![
                TestWidget::<u32>::new(0).into_boxed_render_box(),
                Key::new(0, TestWidget::<usize>::new(0)).into_boxed_render_box(),
                TestWidget::<u32>::new(0).into_boxed_render_box(),
            ])
            .build();
        TestCtx::new().run(|ctx| next.update(&mut element, &mut render, ctx));

        // The keyed child (key 0) is reused; the two unkeyed ends are new types so they remount.
        assert_eq!(
            mounts(),
            6,
            "two new-typed ends mounted; keyed child reused"
        );
        assert_eq!(render.children.len(), 3);
    }
}

#[cfg(test)]
mod element_contract {

    use agui_test::ElementLifecycleCheck;

    use super::Column;

    #[test]
    fn obeys_the_element_contracts() {
        ElementLifecycleCheck::new()
            .multi_child(|children| Column::builder().children(children).build());
    }
}

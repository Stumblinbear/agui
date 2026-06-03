use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    constraints::Constraints,
    context::{Dispatch, UpdateCtx},
    element::{Element, ElementNode},
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    render_object::{MountCtx, RenderNode, RenderObject, box_layout::RenderBox},
    paint::PaintContext,
    routing_id::RoutingId,
    size::Size,
    text_baseline::TextBaseline,
    widget::Widget,
};

pub struct LayoutBuilder<F, Child> {
    builder: Rc<F>,

    _phantom: PhantomData<Child>,
}

impl<F, Child> LayoutBuilder<F, Child>
where
    F: Fn(Constraints) -> Child,
{
    pub fn new(builder: F) -> Self {
        Self {
            builder: Rc::new(builder),

            _phantom: PhantomData,
        }
    }
}

type RetainedChild<Child> = Rc<RefCell<Option<(ElementNode<<Child as Widget>::Element>, Child)>>>;

pub struct LayoutBuilderElement<Child>
where
    Child: Widget,
{
    child_widget: RetainedChild<Child>,

    builder: Rc<dyn Fn(Constraints) -> Child::Render>,
}

impl<Child> Element for LayoutBuilderElement<Child> where Child: Widget + 'static {}

impl<F, Child> Widget for LayoutBuilder<F, Child>
where
    F: Fn(Constraints) -> Child + 'static,
    Child: Widget + 'static,
    Child::Render: RenderBox,
{
    type Element = LayoutBuilderElement<Child>;

    type Render = RenderLayoutBuilder<Child::Render>;

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        let child_widget = Rc::<RefCell<Option<(ElementNode<Child::Element>, Child)>>>::default();

        let builder = {
            let builder = Rc::clone(&self.builder);
            let child_widget = Rc::clone(&child_widget);

            let scheduler = ctx.deferred_scheduler();
            let routing_path = ctx.routing_path();
            let provide_scope = ctx.provide_scope().clone();

            Rc::new(move |constraints| {
                let mut routing_path = routing_path.to_vec();

                let child = (builder)(constraints);

                // Re-derive an owned scheduler per layout: layout runs outside the build frame, so
                // the subtree built here borrows this captured handle to keep spawning tasks.
                let mut scheduler = scheduler.deferred();

                // TODO(trevin): try to use the element of the previously created child
                let element = child.create_element(&mut UpdateCtx::new(
                    &mut *scheduler,
                    &mut routing_path,
                    provide_scope.clone(),
                ));

                let child_render = child.create_render_object(&element);

                child_widget.replace(Some((ElementNode::new(element), child)));

                child_render
            })
        };

        LayoutBuilderElement {
            child_widget,

            builder,
        }
    }

    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
        if !Rc::ptr_eq(&self.builder, &old.builder) {
            element.child_widget.replace(None);

            element.builder = {
                let builder = Rc::clone(&self.builder);
                let child_widget = Rc::clone(&element.child_widget);

                let scheduler = ctx.deferred_scheduler();
                let routing_path = ctx.routing_path();
                let provide_scope = ctx.provide_scope().clone();

                Rc::new(move |constraints| {
                    let child = (builder)(constraints);

                    let mut routing_path = routing_path.to_vec();

                    // Re-derive an owned scheduler per layout: layout runs outside the build frame,
                    // so the subtree built here borrows this captured handle to keep spawning tasks.
                    let mut scheduler = scheduler.deferred();

                    // TODO(trevin): try to use the element of the previously created child
                    let element = child.create_element(&mut UpdateCtx::new(
                        &mut *scheduler,
                        &mut routing_path,
                        provide_scope.clone(),
                    ));

                    let child_render = child.create_render_object(&element);

                    child_widget.replace(Some((ElementNode::new(element), child)));

                    child_render
                })
            };
        }
    }

    fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
        let mut child_widget = element.child_widget.borrow_mut();

        let Some((child_element, child)) = child_widget.as_mut() else {
            panic!("child was dispatched to before being laid out");
        };

        child.dispatch(&mut child_element.element, path, action)
    }

    fn create_render_object(&self, element: &Self::Element) -> Self::Render {
        RenderLayoutBuilder {
            builder: Rc::clone(&element.builder),

            old_constraints: Constraints::default(),

            child_render: None,
        }
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        if !Rc::ptr_eq(&element.builder, &render_object.builder) {
            // TODO(trevin): mark for re-layout
            render_object.builder = Rc::clone(&element.builder);

            render_object.child_render.take();
        }

        let child_widget = element.child_widget.borrow();

        if let Some((child_element, child)) = child_widget.as_ref() {
            if let Some(child_render) = &mut render_object.child_render {
                child.update_render_object(&child_element.element, &mut child_render.object);
            }
        } else if render_object.child_render.is_some() {
            // TODO(trevin): mark for re-layout
            render_object.child_render = None;
        }
    }
}

pub struct RenderLayoutBuilder<Child> {
    builder: Rc<dyn Fn(Constraints) -> Child>,

    old_constraints: Constraints,

    child_render: Option<RenderNode<Child, Option<Size>>>,
}

impl<Child> RenderObject for RenderLayoutBuilder<Child>
where
    Child: RenderBox,
{
    fn mount(&mut self, _: &mut MountCtx) {}

    fn unmount(&mut self, ctx: &mut MountCtx) {
        if let Some(mut child_render) = self.child_render.take() {
            child_render.unmount(ctx);
        }
    }
}

impl<Child> RenderBox for RenderLayoutBuilder<Child>
where
    Child: RenderBox,
{
    fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn measure(&self, _: Constraints) -> Size {
        Size::ZERO
    }

    fn layout(&mut self, constraints: Constraints) -> Size {
        if self.child_render.is_none() || self.old_constraints != constraints {
            self.old_constraints = constraints;

            let child = (self.builder)(constraints);

            self.child_render.replace(RenderNode::new(child));
        }

        if let Some(child_render) = self.child_render.as_mut() {
            let size = child_render.layout_and_get_size(constraints);
            child_render.parent_data = Some(size);
            size
        } else {
            Size::ZERO
        }
    }

    fn measure_baseline(&self, _: Constraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, result: &mut HitTestResult, offset: Offset) -> HitTest {
        if let Some(child_render) = self.child_render.as_ref() {
            child_render.hit_test(result, offset)
        } else {
            HitTest::Pass
        }
    }

    fn paint(&mut self, ctx: &mut PaintContext) {
        if let Some(child_render) = self.child_render.as_mut() {
            child_render.paint(ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use agui_core::{
        render_object::RenderLeaf, task::TaskHandle, test_harness::TestHarness, widget::AsAnyWidget,
    };

    use super::*;
    use crate::sized_box::SizedBox;

    #[test]
    fn calls_closure_during_layout() {
        let build_count = Rc::new(RefCell::new(0));

        let layout_builder = LayoutBuilder::new({
            let build_count = Rc::clone(&build_count);

            move |constraints| {
                *build_count.borrow_mut() += 1;

                if constraints.max_width() > 100.0 {
                    SizedBox::expand().into_boxed_render_box()
                } else {
                    SizedBox::shrink().into_boxed_render_box()
                }
            }
        });

        let mut render_object =
            layout_builder.create_render_object(&TestHarness::mount(&layout_builder).root.element);
        render_object.layout(Constraints::new(0, 50, 0, 50));
        assert_eq!(*build_count.borrow(), 1);
        assert_eq!(
            render_object.child_render.as_ref().unwrap().parent_data,
            Some(Size::new(0.0, 0.0))
        );

        render_object.layout(Constraints::new(0, 150, 0, 150));
        assert_eq!(*build_count.borrow(), 2);
        assert_eq!(
            render_object.child_render.as_ref().unwrap().parent_data,
            Some(Size::new(150.0, 150.0))
        );
    }

    /// A child that spawns a task on mount and stashes its handle so the task outlives the build.
    struct SpawnOnMount;

    struct SpawnOnMountElement {
        _handle: Option<TaskHandle>,
    }

    impl Element for SpawnOnMountElement {}

    impl Widget for SpawnOnMount {
        type Element = SpawnOnMountElement;

        type Render = RenderLeaf;

        fn create_element(&self, ctx: &mut UpdateCtx) -> SpawnOnMountElement {
            let handle = ctx.spawn(|task| async move { task.send(1_u32) }).ok();

            SpawnOnMountElement { _handle: handle }
        }

        fn update(&self, _: &mut SpawnOnMountElement, _: &Self, _: &mut UpdateCtx) {}

        fn create_render_object(&self, _: &SpawnOnMountElement) -> RenderLeaf {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &SpawnOnMountElement, _: &mut RenderLeaf) {}
    }

    #[test]
    fn subtree_can_spawn_tasks_during_layout() {
        // The child is built during layout, not during the LayoutBuilder's own build. It still gets
        // a working scheduler (the deferred handle captured at mount) and posts a message back.
        let layout_builder = LayoutBuilder::new(|_| SpawnOnMount);

        let mut harness = TestHarness::mount(&layout_builder);

        let mut render_object = layout_builder.create_render_object(&harness.root.element);
        render_object.layout(Constraints::new(0, 50, 0, 50));

        harness.task_runner.run_to_completion();

        assert_eq!(
            harness.task_runner.messages().count(),
            1,
            "the subtree spawned a task during layout that posted one message"
        );
    }
}

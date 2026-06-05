use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use typed_floats::{Positive, PositiveFinite};

use agui_core::prelude::{element::*, render_object::*};

/// A widget that builds its child from the constraints handed to it.
///
/// The closure is called with the [`Constraints`] this widget is laid out under and returns the child
/// widget to show for them. It runs during layout, after the constraints are known, so the child can
/// depend on the space available. The closure is re-run whenever the incoming constraints change. The
/// child it returns is laid out within those same constraints, and this widget takes the size the child
/// reports.
///
/// The closure may return a different child for different constraints, including a different concrete
/// widget type, so the return type is typically a boxed widget such as the one produced by
/// [`into_boxed_render_box`](agui_core::widget::AsAnyWidget::into_boxed_render_box).
pub struct LayoutBuilder<F, Child> {
    builder: Rc<F>,

    _phantom: PhantomData<Child>,
}

impl<F, Child> LayoutBuilder<F, Child>
where
    F: Fn(BoxConstraints) -> Child,
{
    pub fn new(builder: F) -> Self {
        Self {
            builder: Rc::new(builder),

            _phantom: PhantomData,
        }
    }
}

/// The child built during layout, shared so the element can dispatch into it while the render object
/// owns its rebuilding. The child is absent until the first layout has run the builder.
type RetainedChild<Child> = Rc<RefCell<Option<(ElementNode<<Child as Widget>::Element>, Child)>>>;

/// The reconcile the render object runs during layout. It builds the child for the current
/// constraints and reconciles the retained subtree, `R` being the child's render type, in place when
/// the new child matches the old in type and key, replacing and remounting it otherwise.
type BuildClosure<R> = Rc<
    dyn Fn(
        &mut LayoutCtx,
        BoxConstraints,
        Option<&PaintScope>,
        &mut Option<RenderNode<R, Option<Size>>>,
    ),
>;

pub struct LayoutBuilderElement<Child>
where
    Child: Widget,
{
    child_widget: RetainedChild<Child>,

    builder: BuildClosure<Child::Render>,
}

impl<Child> Element for LayoutBuilderElement<Child> where Child: Widget + 'static {}

impl<F, Child> Widget for LayoutBuilder<F, Child>
where
    F: Fn(BoxConstraints) -> Child + 'static,
    Child: Widget + 'static,
    Child::Render: RenderBox,
{
    type Element = LayoutBuilderElement<Child>;

    type Render = RenderLayoutBuilder<Child::Render>;

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        let child_widget = RetainedChild::<Child>::default();

        let builder = build_closure(&self.builder, &child_widget, ctx);

        LayoutBuilderElement {
            child_widget,

            builder,
        }
    }

    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
        if !Rc::ptr_eq(&self.builder, &old.builder) {
            // Keep the retained child: the next layout reconciles it in place against the new closure
            // rather than discarding it.
            element.builder = build_closure(&self.builder, &element.child_widget, ctx);
        }
    }

    fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
        let mut child_widget = element.child_widget.borrow_mut();

        let Some((child_element, child)) = child_widget.as_mut() else {
            panic!("child was dispatched to before being laid out");
        };

        child.dispatch(&mut child_element.element, path, action);
    }

    fn create_render_object(&self, element: &Self::Element) -> Self::Render {
        RenderLayoutBuilder {
            builder: Rc::clone(&element.builder),

            old_constraints: BoxConstraints::default(),

            needs_build: false,

            layout_scope: LayoutScope::detached(),

            child_render: None,

            paint_scope: None,
        }
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        if !Rc::ptr_eq(&element.builder, &render_object.builder) {
            // The build logic changed, so the child must be rebuilt even if the constraints are
            // unchanged. `needs_build` forces the re-run, and marking the enclosing boundary
            // schedules the layout that performs it; the retained subtree is reconciled there.
            render_object.builder = Rc::clone(&element.builder);

            render_object.needs_build = true;
            render_object.layout_scope.mark_needs_layout();
        }
    }
}

/// Builds the closure the render object runs during layout to bring the child up to date for a set of
/// constraints, reusing the retained subtree where it can and replacing it where it cannot.
fn build_closure<F, Child>(
    builder: &Rc<F>,
    child_widget: &RetainedChild<Child>,
    ctx: &mut UpdateCtx,
) -> BuildClosure<Child::Render>
where
    F: Fn(BoxConstraints) -> Child + 'static,
    Child: Widget + 'static,
    Child::Render: RenderBox,
{
    let builder = Rc::clone(builder);
    let child_widget = Rc::clone(child_widget);

    let scheduler = ctx.deferred_scheduler();
    let routing_path = ctx.routing_path();
    let provide_scope = ctx.provide_scope().clone();

    Rc::new(
        move |ctx: &mut LayoutCtx,
              constraints: BoxConstraints,
              paint_scope: Option<&PaintScope>,
              slot: &mut Option<RenderNode<Child::Render, Option<Size>>>| {
            let new_child = (builder)(constraints);

            // Layout runs outside the build frame, so the element built or reconciled here borrows an
            // owned scheduler handle, derived from the one captured at build, to keep spawning tasks.
            let mut scheduler = scheduler.deferred();
            let mut routing_path = routing_path.to_vec();
            let mut update =
                UpdateCtx::new(&mut *scheduler, &mut routing_path, provide_scope.clone());

            let mut retained = child_widget.borrow_mut();

            // Reuse the retained subtree when the new child can update it in place.
            if let (Some((node, old_child)), Some(child_render)) =
                (retained.as_mut(), slot.as_mut())
                && new_child.is_same_type(old_child)
                && new_child.key() == old_child.key()
            {
                new_child.update(&mut node.element, old_child, &mut update);
                new_child.update_render_object(&node.element, &mut child_render.object);

                *old_child = new_child;

                return;
            }

            // Otherwise discard the retained subtree, unmounting it first when it was mounted, and
            // build a fresh one.
            if let (Some(mut old), Some(paint_scope)) = (slot.take(), paint_scope) {
                ctx.mount(paint_scope, |mount| old.unmount(mount));
            }

            let element = new_child.create_element(&mut update);
            let mut child_render = RenderNode::new(new_child.create_render_object(&element));

            if let Some(paint_scope) = paint_scope {
                ctx.mount(paint_scope, |mount| child_render.mount(mount));
            }

            *slot = Some(child_render);
            *retained = Some((ElementNode::new(element), new_child));
        },
    )
}

pub struct RenderLayoutBuilder<Child> {
    builder: BuildClosure<Child>,

    old_constraints: BoxConstraints,

    needs_build: bool,

    /// The relayout boundary this widget was last laid out under, marked when the builder changes to
    /// schedule the layout that reruns it. Detached until the first layout.
    layout_scope: LayoutScope,

    child_render: Option<RenderNode<Child, Option<Size>>>,

    paint_scope: Option<PaintScope>,
}

impl<Child> RenderObject for RenderLayoutBuilder<Child>
where
    Child: RenderBox,
{
    fn mount(&mut self, ctx: &mut MountCtx) {
        self.paint_scope = Some(ctx.paint_scope().clone());
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        if let Some(mut child_render) = self.child_render.take() {
            child_render.unmount(ctx);
        }

        self.paint_scope.take();
    }

    fn update_compositing_bits(&mut self) -> bool {
        if let Some(child_render) = self.child_render.as_mut() {
            child_render.update_compositing_bits()
        } else {
            false
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

    fn measure(&self, _: BoxConstraints) -> Size {
        Size::ZERO
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.layout_scope = ctx.scope().clone();

        if self.child_render.is_none() || self.needs_build || self.old_constraints != constraints {
            self.old_constraints = constraints;
            self.needs_build = false;

            let builder = Rc::clone(&self.builder);

            builder(
                ctx,
                constraints,
                self.paint_scope.as_ref(),
                &mut self.child_render,
            );
        }

        if let Some(child_render) = self.child_render.as_mut() {
            let size = child_render.layout_and_get_size(ctx, constraints);
            child_render.parent_data = Some(size);
            size
        } else {
            Size::ZERO
        }
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
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

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        if let Some(child_render) = self.child_render.as_mut() {
            child_render.paint(ctx, offset);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use agui_core::{
        paint::compositing::{ContainerLayer, LayerHandle},
        pipeline::{PipelineOwner, layout::BoundaryContent},
        prelude::{element::*, render_object::*},
        test_harness::TestHarness,
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
        render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 50, 0, 50),
        );
        assert_eq!(*build_count.borrow(), 1);
        assert_eq!(
            render_object.child_render.as_ref().unwrap().parent_data,
            Some(Size::new(0.0, 0.0))
        );

        render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 150, 0, 150),
        );
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

        type Render = ();

        fn create_element(&self, ctx: &mut UpdateCtx) -> SpawnOnMountElement {
            let handle = ctx.spawn(|task| async move { task.send(1_u32) }).ok();

            SpawnOnMountElement { _handle: handle }
        }

        fn update(&self, _: &mut SpawnOnMountElement, _: &Self, _: &mut UpdateCtx) {}

        fn create_render_object(&self, _: &SpawnOnMountElement) -> Self::Render {}

        fn update_render_object(&self, _: &SpawnOnMountElement, _: &mut Self::Render) {}
    }

    #[test]
    fn subtree_can_spawn_tasks_during_layout() {
        // The child is built during layout, not during the LayoutBuilder's own build. It still gets
        // a working scheduler (the deferred handle captured at mount) and posts a message back.
        let layout_builder = LayoutBuilder::new(|_| SpawnOnMount);

        let mut harness = TestHarness::mount(&layout_builder);

        let mut render_object = layout_builder.create_render_object(&harness.root.element);
        render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 50, 0, 50),
        );

        harness.task_runner.run_to_completion();

        assert_eq!(
            harness.task_runner.messages().count(),
            1,
            "the subtree spawned a task during layout that posted one message"
        );
    }

    /// Shared tallies of a [`MountProbe`]'s render-object lifecycle, so a test can tell a reused
    /// subtree (reconciled in place) from a replaced one (unmounted and remounted).
    #[derive(Clone, Default)]
    struct Counts {
        mounts: Rc<Cell<usize>>,
        unmounts: Rc<Cell<usize>>,
        updates: Rc<Cell<usize>>,
    }

    /// A leaf that records its render object's mounts, unmounts, and in-place reconciles.
    struct MountProbe {
        counts: Counts,
    }

    struct MountProbeElement;

    impl Element for MountProbeElement {}

    impl Widget for MountProbe {
        type Element = MountProbeElement;

        type Render = MountProbeRender;

        fn create_element(&self, _: &mut UpdateCtx) -> MountProbeElement {
            MountProbeElement
        }

        fn update(&self, _: &mut MountProbeElement, _: &Self, _: &mut UpdateCtx) {}

        fn create_render_object(&self, _: &MountProbeElement) -> MountProbeRender {
            MountProbeRender {
                counts: self.counts.clone(),
            }
        }

        fn update_render_object(&self, _: &MountProbeElement, render: &mut MountProbeRender) {
            render.counts.updates.set(render.counts.updates.get() + 1);
        }
    }

    struct MountProbeRender {
        counts: Counts,
    }

    impl RenderObject for MountProbeRender {
        fn mount(&mut self, _: &mut MountCtx) {
            self.counts.mounts.set(self.counts.mounts.get() + 1);
        }

        fn unmount(&mut self, _: &mut MountCtx) {
            self.counts.unmounts.set(self.counts.unmounts.get() + 1);
        }

        fn update_compositing_bits(&mut self) -> bool {
            false
        }
    }

    impl RenderBox for MountProbeRender {
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

        fn measure(&self, constraints: BoxConstraints) -> Size {
            constraints.smallest()
        }

        fn layout(&mut self, _: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
            constraints.smallest()
        }

        fn measure_baseline(
            &self,
            _: BoxConstraints,
            _: TextBaseline,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
            None
        }

        fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
            HitTest::Pass
        }

        fn paint(&mut self, _: &mut PaintCtx, _: Offset) {}
    }

    /// Drives `layout_builder` as the root of a real pipeline, so layout reaches the paint registry
    /// and the subtree built during layout is mounted.
    fn owner_for<W>(layout_builder: &W) -> PipelineOwner
    where
        W: Widget,
        W::Render: RenderBox,
    {
        let harness = TestHarness::mount(layout_builder);
        let render = layout_builder.create_render_object(&harness.root.element);
        let content: BoundaryContent = Rc::new(RefCell::new(render));

        PipelineOwner::new(content, LayerHandle::new(ContainerLayer::new()))
    }

    #[test]
    fn reuses_its_subtree_across_a_constraint_change() {
        let counts = Counts::default();

        let layout_builder = LayoutBuilder::new({
            let counts = counts.clone();

            move |_| {
                MountProbe {
                    counts: counts.clone(),
                }
                .into_boxed_render_box()
            }
        });

        let mut owner = owner_for(&layout_builder);

        owner.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();
        assert_eq!(counts.mounts.get(), 1, "the subtree was mounted once");
        assert_eq!(counts.updates.get(), 0);

        // The constraints change but the child keeps its type, so the subtree is reconciled in place
        // rather than discarded and remounted.
        owner.resize(BoxConstraints::new(0, 50, 0, 50));
        owner.flush_layout();
        assert_eq!(counts.mounts.get(), 1, "the same-type child was reused");
        assert_eq!(counts.unmounts.get(), 0);
        assert_eq!(
            counts.updates.get(),
            1,
            "the reused child was reconciled in place"
        );
    }

    #[test]
    fn replaces_its_subtree_when_the_child_type_changes() {
        let counts = Counts::default();

        // A probe above the threshold, a plain box below it, so crossing the threshold changes the
        // child's type.
        let layout_builder = LayoutBuilder::new({
            let counts = counts.clone();

            move |constraints| {
                if constraints.max_width() > 75.0 {
                    MountProbe {
                        counts: counts.clone(),
                    }
                    .into_boxed_render_box()
                } else {
                    SizedBox::shrink().into_boxed_render_box()
                }
            }
        });

        let mut owner = owner_for(&layout_builder);

        owner.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();
        assert_eq!(counts.mounts.get(), 1);
        assert_eq!(counts.unmounts.get(), 0);

        // Cross the threshold: the probe's type no longer matches, so its subtree is unmounted.
        owner.resize(BoxConstraints::new(0, 50, 0, 50));
        owner.flush_layout();
        assert_eq!(counts.mounts.get(), 1, "the replacement was not the probe");
        assert_eq!(
            counts.unmounts.get(),
            1,
            "the probe's subtree was unmounted"
        );

        // Cross back: a fresh probe is built and mounted.
        owner.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();
        assert_eq!(counts.mounts.get(), 2, "a fresh probe was mounted");
    }

    #[test]
    fn rebuilds_when_the_callback_changes_without_a_resize() {
        let builds = Rc::new(Cell::new(0));

        let widget_for = |builds: &Rc<Cell<usize>>| {
            let builds = Rc::clone(builds);

            LayoutBuilder::new(move |_| {
                builds.set(builds.get() + 1);
                SizedBox::shrink().into_boxed_render_box()
            })
        };

        let widget_a = widget_for(&builds);
        let mut harness = TestHarness::mount(&widget_a);
        let render = widget_a.create_render_object(&harness.root.element);
        let content: BoundaryContent = Rc::new(RefCell::new(render));
        let mut owner =
            PipelineOwner::new(Rc::clone(&content), LayerHandle::new(ContainerLayer::new()));

        owner.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();
        assert_eq!(builds.get(), 1);

        owner.flush_layout();
        assert_eq!(builds.get(), 1, "an unmarked frame reuses the layout");

        // The widget rebuilds with a new callback at the same constraints. This must schedule a
        // relayout that reruns the builder, the way Flutter's markNeedsLayout does.
        let widget_b = widget_for(&builds);
        harness.update(&widget_a, &widget_b);
        owner.update(|root| {
            let render = root
                .as_any_mut()
                .downcast_mut::<RenderLayoutBuilder<Box<dyn AnyRenderBox>>>()
                .expect("the root is the layout builder");

            widget_b.update_render_object(&harness.root.element, render);
        });

        owner.flush_layout();
        assert_eq!(
            builds.get(),
            2,
            "the callback change scheduled a relayout that reran the builder"
        );
    }
}

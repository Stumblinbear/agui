use std::{any::TypeId, cell::RefCell, marker::PhantomData, rc::Rc};

use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    key::AnyKeyable,
    pipeline::{layout::LayoutPipeline, paint::PaintPipeline},
    prelude::{element::*, render_object::*},
};

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
    builder: F,

    _phantom: PhantomData<Child>,
}

impl<F, Child> LayoutBuilder<F, Child>
where
    F: Fn(BoxConstraints) -> Child,
{
    pub fn new(builder: F) -> Self {
        Self {
            builder,

            _phantom: PhantomData,
        }
    }
}

/// The child built during layout, shared so the element can dispatch into it while the render object
/// owns its rebuilding. The child is absent until the first layout has run the builder.
type RetainedChild<Child> = Rc<RefCell<Option<RetainedNode<Child>>>>;

/// A retained child element paired with the type and key its widget reported at build, so the next
/// layout can decide whether to reconcile it in place or replace it.
struct RetainedNode<Child: Widget> {
    node: ElementNode<Child::Element>,
    type_id: TypeId,
    key: Option<Box<dyn AnyKeyable>>,
}

/// The reconcile the render object runs during layout. It builds the child for the current
/// constraints and reconciles the retained subtree, `R` being the child's render type, in place when
/// the new child matches the old in type and key, replacing and remounting it otherwise.
type BuildClosure<R> = Rc<
    dyn Fn(&mut LayoutCtx, BoxConstraints, &PaintScope, &mut Option<RenderNode<R, Option<Size>>>),
>;

pub struct LayoutBuilderElement<F, Child>
where
    Child: Widget,
{
    child_widget: RetainedChild<Child>,

    /// The closure the element was last built from.
    source: F,

    builder: BuildClosure<Child::Render>,
}

impl<F, Child> Element for LayoutBuilderElement<F, Child>
where
    F: 'static,
    Child: Widget,
    Child::Render: RenderBox,
{
    type Render = RenderLayoutBuilder<Child::Render>;

    fn dispatch(&mut self, render: &mut Self::Render, path: &[RoutingId], action: Dispatch) {
        let mut child_widget = self.child_widget.borrow_mut();

        let Some(retained) = child_widget.as_mut() else {
            panic!("child was dispatched to before being laid out");
        };

        let child_render = render
            .child_render
            .as_mut()
            .expect("child was dispatched to before being laid out");

        retained
            .node
            .element
            .dispatch(&mut child_render.object, path, action);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        let mut node = d.node_for::<Self>();

        let child_widget = self.child_widget.borrow();

        if let Some(retained) = child_widget.as_ref() {
            node = node.child(|d| retained.node.element.describe(d));
        }

        node.finish()
    }
}

impl<F, Child> Widget for LayoutBuilder<F, Child>
where
    F: Fn(BoxConstraints) -> Child + 'static,
    F: Clone,
    Child: Widget + 'static,
    Child::Render: RenderBox,
{
    type Element = LayoutBuilderElement<F, Child>;

    type Render = RenderLayoutBuilder<Child::Render>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let source = self.builder;
        let child_widget = RetainedChild::<Child>::default();

        let builder = build_closure(&source, &child_widget, ctx);

        let render_object = RenderLayoutBuilder {
            builder: Rc::clone(&builder),

            old_constraints: BoxConstraints::default(),

            needs_build: false,

            layout_scope: LayoutScope::detached(),
            paint_scope: PaintScope::detached(),

            child_render: None,
        };

        let element = LayoutBuilderElement {
            child_widget,

            source,

            builder,
        };

        (element, render_object)
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        // TODO(trevin): Is there some way to check equality of the builder?

        element.source = self.builder;

        // Keep the retained child: the next layout reconciles it in place against the new closure
        // rather than discarding it.
        element.builder = build_closure(&element.source, &element.child_widget, ctx);

        // The build logic changed, so the child must be rebuilt even if the constraints are
        // unchanged. `needs_build` forces the re-run, and marking the enclosing boundary
        // schedules the layout that performs it; the retained subtree is reconciled there.
        render_object.builder = Rc::clone(&element.builder);

        render_object.needs_build = true;
        render_object.layout_scope.mark_needs_layout();
    }
}

/// Builds the closure the render object runs during layout to bring the child up to date for a set of
/// constraints, reusing the retained subtree where it can and replacing it where it cannot.
fn build_closure<F, Child>(
    builder: &F,
    child_widget: &RetainedChild<Child>,
    ctx: &mut UpdateCtx,
) -> BuildClosure<Child::Render>
where
    F: Fn(BoxConstraints) -> Child + 'static,
    F: Clone,
    Child: Widget + 'static,
    Child::Render: RenderBox,
{
    let builder = builder.clone();
    let child_widget = Rc::clone(child_widget);

    let scheduler = ctx.deferred_scheduler();
    let routing_path = ctx.routing_path();
    let provide_scope = ctx.provide_scope().clone();
    let build_scope = ctx.build_scope().clone();

    Rc::new(
        move |ctx: &mut LayoutCtx,
              constraints: BoxConstraints,
              paint_scope: &PaintScope,
              slot: &mut Option<RenderNode<Child::Render, Option<Size>>>| {
            let new_child = (builder)(constraints);

            // Layout runs outside the build frame, so the element built or reconciled here borrows an
            // owned scheduler handle, derived from the one captured at build, to keep spawning tasks.
            let mut scheduler = scheduler.deferred();
            let mut routing_path = routing_path.within().to_vec();
            // This build runs during layout and mounts the child it produces explicitly through the
            // layout-time mount below, not through the reconcile path, so it carries its own pipeline.
            let mut build_paint = PaintPipeline::default();
            let build_layout = LayoutPipeline::default();
            let mut update = UpdateCtx::new(
                &mut *scheduler,
                &mut routing_path,
                &provide_scope,
                &build_scope,
                &build_layout,
                &mut build_paint,
                paint_scope,
            );

            let mut retained = child_widget.borrow_mut();

            let new_type_id = new_child.widget_type_id();
            let new_key = new_child.key().map(AnyKeyable::dyn_clone);

            // Reuse the retained subtree when the new child can update it in place.
            if let (Some(node), Some(child_render)) = (retained.as_mut(), slot.as_mut())
                && node.type_id == new_type_id
                && key_eq(new_key.as_deref(), node.key.as_deref())
            {
                node.key = new_key;

                new_child.update(
                    &mut node.node.element,
                    &mut child_render.object,
                    &mut update,
                );

                return;
            }

            // Otherwise discard the retained subtree, unmounting it first when it was mounted, and
            // build a fresh one.
            if let Some(mut old) = slot.take() {
                ctx.mount(paint_scope, |mount| old.unmount(mount));
            }

            let (element, child_object) = new_child.create(&mut update);
            let mut child_render = RenderNode::new(child_object);

            ctx.mount(paint_scope, |mount| child_render.mount(mount));

            // The fresh subtree's compositing bits sit at their defaults; schedule a recompute so a
            // compositing descendant paints into its layer rather than as flat drawing.
            paint_scope.mark_needs_compositing_bits_update();

            *slot = Some(child_render);
            *retained = Some(RetainedNode {
                node: ElementNode::new(element),
                type_id: new_type_id,
                key: new_key,
            });
        },
    )
}

fn key_eq(a: Option<&dyn AnyKeyable>, b: Option<&dyn AnyKeyable>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a == b,
        (None, None) => true,
        _ => false,
    }
}

pub struct RenderLayoutBuilder<Child> {
    builder: BuildClosure<Child>,

    old_constraints: BoxConstraints,

    needs_build: bool,

    /// The relayout boundary this widget was last laid out under, marked when the builder changes to
    /// schedule the layout that reruns it. Detached until the first layout.
    layout_scope: LayoutScope,
    paint_scope: PaintScope,

    child_render: Option<RenderNode<Child, Option<Size>>>,
}

impl<Child> RenderObject for RenderLayoutBuilder<Child>
where
    Child: RenderBox,
{
    fn mount(&mut self, ctx: &mut MountCtx) {
        self.paint_scope = ctx.paint_scope().clone();
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        if let Some(mut child_render) = self.child_render.take() {
            child_render.unmount(ctx);
        }

        self.paint_scope = PaintScope::detached();
    }

    fn update_compositing_bits(&mut self) -> bool {
        if let Some(child_render) = self.child_render.as_mut() {
            child_render.update_compositing_bits()
        } else {
            false
        }
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        let mut node = d.node_for::<Self>();

        if let Some(child_render) = self.child_render.as_ref() {
            node = node.child(|d| child_render.describe(d));
        }

        node.finish()
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

            builder(ctx, constraints, &self.paint_scope, &mut self.child_render);
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
        element::BuildScope,
        paint::compositing::{LayerHandle, OffsetLayer},
        pipeline::{PipelineOwner, layout::BoundaryContent},
        prelude::{element::*, render_object::*},
        provide::ProvideScope,
        test_harness::{TestTaskRunner, mount_view, with_ctx},
        view::ViewHandle,
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

        let (_, mut render_object) = with_ctx(|ctx| layout_builder.create(ctx));
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

    impl Element for SpawnOnMountElement {
        type Render = ();
    }

    impl Widget for SpawnOnMount {
        type Element = SpawnOnMountElement;

        type Render = ();

        fn create(self, ctx: &mut UpdateCtx) -> (SpawnOnMountElement, ()) {
            let handle = ctx.spawn(|task| async move { task.send(1_u32) }).ok();

            (SpawnOnMountElement { _handle: handle }, ())
        }

        fn update(self, _: &mut SpawnOnMountElement, (): &mut (), _: &mut UpdateCtx) {}
    }

    #[test]
    fn subtree_can_spawn_tasks_during_layout() {
        // The child is built during layout, not during the LayoutBuilder's own build. It still gets
        // a working scheduler (the deferred handle captured at mount) and posts a message back.
        let layout_builder = LayoutBuilder::new(|_| SpawnOnMount);

        let mut tasks = TestTaskRunner::new();

        let (_, mut render_object) = {
            let provide = ProvideScope::new();
            let mut path = Vec::new();
            let mut scheduler = tasks.scheduler();
            let build_scope = BuildScope::detached();
            let mut paint = PaintPipeline::default();
            let layout = LayoutPipeline::default();
            let paint_scope = PaintScope::detached();
            let mut ctx = UpdateCtx::new(
                &mut scheduler,
                &mut path,
                &provide,
                &build_scope,
                &layout,
                &mut paint,
                &paint_scope,
            );

            layout_builder.create(&mut ctx)
        };
        render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 50, 0, 50),
        );

        tasks.run_to_completion();

        assert_eq!(
            tasks.messages().count(),
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

    impl Element for MountProbeElement {
        type Render = MountProbeRender;
    }

    impl Widget for MountProbe {
        type Element = MountProbeElement;

        type Render = MountProbeRender;

        fn create(self, _: &mut UpdateCtx) -> (MountProbeElement, MountProbeRender) {
            (
                MountProbeElement,
                MountProbeRender {
                    counts: self.counts,
                },
            )
        }

        fn update(
            self,
            _: &mut MountProbeElement,
            render: &mut MountProbeRender,
            _: &mut UpdateCtx,
        ) {
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
    fn owner_for<W>(layout_builder: W) -> (PipelineOwner, ViewHandle)
    where
        W: Widget,
        W::Element: 'static,
        W::Render: RenderBox,
    {
        mount_view(layout_builder)
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

        let (mut owner, view) = owner_for(layout_builder);

        view.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();
        assert_eq!(counts.mounts.get(), 1, "the subtree was mounted once");
        assert_eq!(counts.updates.get(), 0);

        // The constraints change but the child keeps its type, so the subtree is reconciled in place
        // rather than discarded and remounted.
        view.resize(BoxConstraints::new(0, 50, 0, 50));
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

        let (mut owner, view) = owner_for(layout_builder);

        view.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();
        assert_eq!(counts.mounts.get(), 1);
        assert_eq!(counts.unmounts.get(), 0);

        // Cross the threshold: the probe's type no longer matches, so its subtree is unmounted.
        view.resize(BoxConstraints::new(0, 50, 0, 50));
        owner.flush_layout();
        assert_eq!(counts.mounts.get(), 1, "the replacement was not the probe");
        assert_eq!(
            counts.unmounts.get(),
            1,
            "the probe's subtree was unmounted"
        );

        // Cross back: a fresh probe is built and mounted.
        view.resize(BoxConstraints::new(0, 100, 0, 100));
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
        let (mut element, render) = with_ctx(|ctx| widget_a.create(ctx));
        let mut content: BoundaryContent = Rc::new(RefCell::new(render));

        // Mount the builder as a root layout boundary by hand, so the manual callback change below can
        // mark it for relayout and the flush can rerun the builder.
        let layer = LayerHandle::new(OffsetLayer::new());
        let (mut paint, paint_boundary) = PaintPipeline::new(Rc::clone(&content), layer);
        let layout = LayoutPipeline::default();
        let layout_boundary = layout.register_root(Rc::clone(&content), paint_boundary.scope());
        {
            let scope = paint_boundary.scope();
            let mut ctx = MountCtx::new(&layout, &mut paint, &scope);
            content.mount(&mut ctx);
        }

        layout_boundary.set_constraints(BoxConstraints::new(0, 100, 0, 100));
        layout.flush(&mut paint);
        assert_eq!(builds.get(), 1);

        layout.flush(&mut paint);
        assert_eq!(builds.get(), 1, "an unmarked frame reuses the layout");

        // The widget rebuilds with a new callback at the same constraints. This must schedule a
        // relayout that reruns the builder, the way Flutter's markNeedsLayout does.
        let widget_b = widget_for(&builds);
        {
            let mut root = content.borrow_mut();
            let render = root
                .as_any_mut()
                .downcast_mut::<RenderLayoutBuilder<Box<dyn AnyRenderBox>>>()
                .expect("the root is the layout builder");

            with_ctx(|ctx| widget_b.update(&mut element, render, ctx));
        }

        layout.flush(&mut paint);
        assert_eq!(
            builds.get(),
            2,
            "the callback change scheduled a relayout that reran the builder"
        );
    }
}

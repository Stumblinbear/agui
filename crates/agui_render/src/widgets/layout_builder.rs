use agui_core::tree::{BoxedSlot, NodeHandle};
use typed_floats::{Positive, PositiveFinite};

use crate::{
    prelude::{element::*, render_object::*},
    provide::ProvideScope,
    scheduling::TaskScheduler,
    widget::AnyWidget,
};

/// The child a `LayoutBuilder` produces: a boxed widget rendering `dyn RenderBox`, so the closure may return a
/// different concrete widget for different constraints.
type BoxedChild = Box<dyn AnyWidget<Render = dyn RenderBox>>;

/// A widget that builds its child from the constraints handed to it.
///
/// The closure runs during layout, once the constraints are known, and returns the child to show for them. It
/// reruns whenever the incoming constraints change. The child is laid out within those same constraints, and
/// the widget takes the size the child reports. Box the returned child (for example with
/// [`into_boxed_render_box`](crate::widget::AsAnyWidget::into_boxed_render_box)) so the closure can return a
/// different widget per constraints.
pub struct LayoutBuilder<F> {
    builder: F,
}

impl<F> LayoutBuilder<F>
where
    F: Fn(BoxConstraints) -> BoxedChild,
{
    pub fn new(builder: F) -> Self {
        Self { builder }
    }
}

impl<F> Widget for LayoutBuilder<F>
where
    F: Fn(BoxConstraints) -> BoxedChild + 'static,
{
    type Element = LayoutBuilderElement<F>;

    type Render = RenderLayoutBuilder<F>;

    fn create(self, _ctx: &mut CreateCtx) -> Self::Element {
        LayoutBuilderElement {
            render: RenderObjectCell::new(RenderLayoutBuilder {
                builder: self.builder,
                child: None,
                edge: RenderNode::new(None),
                handle: NodeHandle::default(),
                scope: ProvideScope::default(),
                scheduler: None,
                old_constraints: None,
                needs_build: true,
                layout_scope: LayoutScope::detached(),
            }),
        }
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let render = element.render_object_mut();
        render.builder = self.builder;

        // The closure changed, so the child must be rebuilt even at unchanged constraints; force it and
        // schedule the layout that runs it.
        render.needs_build = true;
        ctx.mark_needs_layout(render.layout_scope);
    }
}

/// The [`Element`] of a [`LayoutBuilder`]. It owns the render object and forwards mount and unmount to it; the
/// render object holds the child the builder produces during layout.
pub struct LayoutBuilderElement<F> {
    render: RenderObjectCell<RenderLayoutBuilder<F>>,
}

// SAFETY: its render object reconciles the one child only through the cursor child operations, and the element
// resolves its render object from its own `RenderObjectCell`.
unsafe impl<F> Element for LayoutBuilderElement<F>
where
    F: Fn(BoxConstraints) -> BoxedChild + 'static,
{
    type Render = RenderLayoutBuilder<F>;

    fn render_object_mut(&mut self) -> &mut Self::Render {
        self.render.get_mut()
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<Self::Render> {
        self.render.render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        let render = self.render.get_mut();
        render.handle = ctx.handle();
        render.scope = ctx.provide();
        // Layout runs outside the build, so capture an owned scheduler handle now for the child built then.
        render.scheduler = Some(ctx.deferred_scheduler());
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        if let Some(child) = &mut self.render.get_mut().child {
            // SAFETY: `child` is this render object's own slot.
            unsafe { ctx.unmount(child) };
        }
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>().finish()
    }
}

/// The render object of a [`LayoutBuilder`]. On a constraint change it reruns the builder, mounts or
/// reconciles the child it returns, lays that child out, and takes its size.
pub struct RenderLayoutBuilder<F> {
    builder: F,
    /// The element of the child built during layout. `None` until the first build.
    child: Option<BoxedSlot<<BoxedChild as Widget>::Element>>,
    edge: RenderNode<dyn RenderBox, Option<Size>>,
    handle: NodeHandle,
    scope: ProvideScope,
    scheduler: Option<Box<dyn TaskScheduler>>,
    old_constraints: Option<BoxConstraints>,
    needs_build: bool,
    layout_scope: LayoutScope,
}

impl<F: 'static> RenderObject for RenderLayoutBuilder<F> {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child(|d| self.edge.describe(d))
            .finish()
    }
}

impl<F> RenderBox for RenderLayoutBuilder<F>
where
    F: Fn(BoxConstraints) -> BoxedChild + 'static,
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

    fn measure(&self, constraints: BoxConstraints) -> Size {
        constraints.smallest()
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.layout_scope = *ctx.scope();

        if self.needs_build || self.old_constraints != Some(constraints) {
            self.old_constraints = Some(constraints);
            self.needs_build = false;

            let handle = self.handle;
            let scope = self.scope;
            let builder = &self.builder;
            let child = &mut self.child;
            let scheduler = self
                .scheduler
                .as_deref_mut()
                .expect("the scheduler is captured at mount");

            let mounted = ctx
                .build_child(handle, scheduler, |ctx| {
                    ctx.with_scope(scope, |ctx| {
                        let widget = builder(constraints);

                        if let Some(slot) = child {
                            // The boxed child reconciles a type change inside its own element, so the mounted
                            // node stays put and the edge needs no rewiring.
                            // SAFETY: `slot` is this render object's own child slot.
                            unsafe {
                                ctx.with_child(slot, |element, ctx| {
                                    Widget::update(widget, ctx, element);
                                });
                            }
                            None
                        } else {
                            let element = ctx.inflate(|ctx| Widget::create(widget, ctx));
                            let mut slot = BoxedSlot::new(element);
                            // SAFETY: `slot` is freshly built and owned here; a `BoxedSlot` is heap, so moving
                            // it below keeps its registered address.
                            let mounted = unsafe { ctx.mount(&mut slot) };
                            *child = Some(slot);
                            Some(mounted)
                        }
                    })
                })
                .expect("the layout builder is present during its own layout");

            if let Some(mounted) = mounted {
                self.edge.set(mounted);
            }
        }

        let size = self.edge.layout_and_get_size(ctx, constraints);
        self.edge.parent_data = Some(size);
        size
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        let Some(size) = self.edge.parent_data else {
            return HitTest::Pass;
        };

        if !size.contains(position) {
            return HitTest::Pass;
        }

        self.edge.hit_test(result, position)
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.edge.update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.edge.paint(ctx, offset);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use typed_floats::{Positive, PositiveFinite};

    use crate::{
        prelude::{element::*, render_object::*},
        test_harness::TestCtx,
        widget::AsAnyWidget,
    };

    use super::LayoutBuilder;

    /// A leaf that lays out to a fixed square, so a test can read the size a `LayoutBuilder` chose.
    struct Fixed {
        size: f32,
    }

    impl Widget for Fixed {
        type Element = LeafElement<RenderFixed>;

        type Render = RenderFixed;

        fn create(self, _ctx: &mut CreateCtx) -> Self::Element {
            LeafElement::new(RenderFixed { size: self.size })
        }

        fn update(self, _ctx: &mut UpdateCtx, element: &mut Self::Element) {
            element.render_object_mut().size = self.size;
        }
    }

    struct RenderFixed {
        size: f32,
    }

    impl RenderObject for RenderFixed {
        fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
            d.node_for::<Self>().finish()
        }
    }

    impl RenderBox for RenderFixed {
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
            Size::new(self.size, self.size)
        }

        fn layout(&mut self, _: &mut LayoutCtx, _: BoxConstraints) -> Size {
            Size::new(self.size, self.size)
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

        fn update_compositing_bits(&mut self) -> bool {
            false
        }

        fn paint(&mut self, _: &mut PaintCtx, _: Offset) {}
    }

    /// The builder runs during layout, once per constraint change, and the child it produces is mounted and
    /// laid out. Run under Miri, this exercises the layout-time mount: the render object reconciles its own
    /// child through a cursor while it is mid-layout-borrow.
    #[test]
    fn builds_its_child_during_layout_and_reruns_on_a_constraint_change() {
        let builds = Rc::new(Cell::new(0));

        let widget = LayoutBuilder::new({
            let builds = Rc::clone(&builds);
            move |constraints: BoxConstraints| {
                builds.set(builds.get() + 1);
                let size = if constraints.max_width().get() > 100.0 {
                    80.0
                } else {
                    20.0
                };
                Fixed { size }.into_boxed_render_box()
            }
        });

        let (mut owner, view) = TestCtx::new().mount_view(widget);

        view.resize(BoxConstraints::new(0, 200, 0, 200));
        owner.flush_layout();
        assert_eq!(
            builds.get(),
            1,
            "the builder ran once during the first layout"
        );

        view.resize(BoxConstraints::new(0, 50, 0, 50));
        owner.flush_layout();
        assert_eq!(
            builds.get(),
            2,
            "the builder reran for the changed constraints"
        );
    }
}

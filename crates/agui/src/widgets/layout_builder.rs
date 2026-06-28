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

                element_handle: NodeHandle::default(),
                scope: ProvideScope::default(),
                scheduler: None,

                layout_scope: LayoutScope::detached(),
                old_constraints: None,
                needs_build: true,

                child: None,
                child_node: RenderNode::new(None),
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

impl<F> LayoutBuilderElement<F> {
    /// This element's render object, by exclusive reference, for the widget's own writes during reconcile.
    pub fn render_object_mut(&mut self) -> &mut RenderLayoutBuilder<F> {
        self.render.get_mut()
    }
}

// SAFETY: its render object reconciles the one child only through the cursor child operations, and the element
// resolves its render object from its own `RenderObjectCell`.
unsafe impl<F> Element for LayoutBuilderElement<F>
where
    F: Fn(BoxConstraints) -> BoxedChild + 'static,
{
    type Render = RenderLayoutBuilder<F>;

    fn render_object_ptr(&self) -> RenderObjectPtr<Self::Render> {
        self.render.render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        let render = self.render.get_mut();
        render.element_handle = ctx.handle();
        render.scope = ctx.provide_scope();
        // Layout runs outside the build, so capture an owned scheduler handle now for the child built then.
        render.scheduler = Some(ctx.deferred_scheduler());

        render.attach(ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.render.get_mut().detach(ctx);

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

    element_handle: NodeHandle,
    scope: ProvideScope,
    scheduler: Option<Box<dyn TaskScheduler>>,

    layout_scope: LayoutScope,

    old_constraints: Option<BoxConstraints>,
    needs_build: bool,

    /// The element of the child built during layout. `None` until the first build.
    child: Option<BoxedSlot<<BoxedChild as Widget>::Element>>,
    child_node: RenderNode<<BoxedChild as Widget>::Render, Option<Size>>,
}

impl<F: 'static> RenderObject for RenderLayoutBuilder<F> {
    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        self.child_node.build_semantics(s);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child(|d| self.child_node.describe(d))
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

            let handle = self.element_handle;
            let scope = self.scope;
            let builder = &self.builder;
            let child = &mut self.child;
            let scheduler = self
                .scheduler
                .as_deref_mut()
                .expect("the scheduler is captured at mount");

            let mounted = ctx
                .build_child(handle, scheduler, |ctx| {
                    ctx.with_provide_scope(scope, |ctx| {
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
                self.child_node.set(mounted);
            }
        }

        let size = self.child_node.layout_and_get_size(ctx, constraints);
        self.child_node.child_data = Some(size);
        size
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        let Some(size) = self.child_node.child_data else {
            return HitTest::Pass;
        };

        if !size.contains(position) {
            return HitTest::Pass;
        }

        self.child_node.hit_test(result, position)
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child_node.update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.child_node.paint(ctx, offset);
    }
}

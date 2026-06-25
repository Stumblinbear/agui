use std::{cell::RefCell, rc::Rc};

use agui::{
    geometry::{Offset, Size},
    prelude::{element::*, render_object::*},
};
use typed_floats::{Positive, PositiveFinite};

#[derive(Default)]
struct ProbeState {
    target: Option<RoutingTarget>,
    constraints: Option<BoxConstraints>,
    size: Option<Size>,
    offset: Option<Offset>,
    layouts: usize,
    paints: usize,
}

/// An observation point on a widget in the tree under test.
///
/// Attach it with [`wrap`](Self::wrap); it then records what its widget was asked to do: the
/// constraints it was laid out under, the size it took, the offset it painted at, and how many times
/// it was laid out and painted. It also captures the widget's routing path, so a test can dispatch to
/// it without naming the path by hand. A probe reflects only what has happened so far:
/// [`size`](Self::size), [`constraints`](Self::constraints), [`offset`](Self::offset), and
/// [`path`](Self::path) panic when asked for something that has not happened yet.
#[derive(Clone, Default)]
pub struct Probe {
    state: Rc<RefCell<ProbeState>>,
}

impl Probe {
    pub fn new() -> Self {
        Self::default()
    }

    /// Wraps `child` in a widget that forwards every operation to it and records the results onto this
    /// probe.
    pub fn wrap<Child>(&self, child: Child) -> Spy<Child> {
        Spy {
            state: Rc::clone(&self.state),
            child,
        }
    }

    /// The number of times the probed widget has been laid out.
    pub fn layouts(&self) -> usize {
        self.state.borrow().layouts
    }

    /// The number of times the probed widget has been painted.
    pub fn paints(&self) -> usize {
        self.state.borrow().paints
    }

    /// The size the probed widget took at its most recent layout.
    ///
    /// # Panics
    ///
    /// Panics if the probed widget has not been laid out.
    pub fn size(&self) -> Size {
        self.state
            .borrow()
            .size
            .expect("the probed widget has not been laid out")
    }

    /// The constraints the probed widget was laid out under at its most recent layout.
    ///
    /// # Panics
    ///
    /// Panics if the probed widget has not been laid out.
    pub fn constraints(&self) -> BoxConstraints {
        self.state
            .borrow()
            .constraints
            .expect("the probed widget has not been laid out")
    }

    /// The offset the probed widget painted at, in the coordinate space of its enclosing boundary.
    ///
    /// # Panics
    ///
    /// Panics if the probed widget has not been painted.
    pub fn offset(&self) -> Offset {
        self.state
            .borrow()
            .offset
            .expect("the probed widget has not been painted")
    }

    /// The routing target that addresses the probed widget for dispatch.
    ///
    /// # Panics
    ///
    /// Panics if the probed widget has not been mounted.
    pub fn target(&self) -> RoutingTarget {
        self.state
            .borrow()
            .target
            .clone()
            .expect("the probed widget has not been mounted")
    }
}

/// The widget a [`Probe`] splices into the tree. It presents its child unchanged and records the
/// child's layout and paint onto the probe.
pub struct Spy<Child> {
    state: Rc<RefCell<ProbeState>>,
    child: Child,
}

impl<Child> Widget for Spy<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderSpy<Child::Render>>;

    type Render = RenderSpy<Child::Render>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        self.state.borrow_mut().target = Some(ctx.routing_target());

        let (element, child_render) = SingleChildElement::new(self.child, ctx);

        (
            element,
            RenderSpy {
                state: self.state,
                child: RenderNode::new(child_render),
            },
        )
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        element.update(self.child, &mut render_object.child.object, ctx);
    }
}

/// The render object of a [`Spy`]: it lays out and paints its child unchanged, recording the result.
pub struct RenderSpy<Child> {
    state: Rc<RefCell<ProbeState>>,
    child: RenderNode<Child>,
}

impl<Child> SingleChildRenderObject for RenderSpy<Child> {
    type Child = Child;

    fn with_child<R>(&self, f: impl FnOnce(&Child) -> R) -> R {
        f(&self.child.object)
    }

    fn with_child_mut<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
        f(&mut self.child.object)
    }
}

impl<Child: RenderBox> RenderObject for RenderSpy<Child> {
    fn mount(&mut self, ctx: &mut MountCtx) {
        self.child.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.child.unmount(ctx);
    }
}

impl<Child: RenderBox> RenderBox for RenderSpy<Child> {
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.min_intrinsic_width(height)
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.max_intrinsic_width(height)
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.min_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.max_intrinsic_height(width)
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        self.child.measure(constraints)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        let size = self.child.layout_and_get_size(ctx, constraints);

        let mut state = self.state.borrow_mut();
        state.constraints = Some(constraints);
        state.size = Some(size);
        state.layouts += 1;

        size
    }

    fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.child.measure_baseline(constraints, baseline)
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.child.distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.child.hit_test(result, position)
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        {
            let mut state = self.state.borrow_mut();
            state.offset = Some(offset);
            state.paints += 1;
        }

        self.child.paint(ctx, offset);
    }
}

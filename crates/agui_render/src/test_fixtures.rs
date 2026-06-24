use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use typed_floats::{Positive, PositiveFinite};

use agui_core::tree::NodeHandle;

use crate::{
    context::{CreateCtx, LayoutCtx, MessageCtx, PaintCtx, UpdateCtx},
    element::{Element, SingleChildElement},
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    key::AnyKeyable,
    pipeline::render_pipeline::DeferredLayoutScope,
    render_object::{
        MultiChildRenderObject, RenderObject, SingleChildRenderObject,
        box_layout::{BoxConstraints, RenderBox},
        node::{MountedChild, RenderNode, RenderObjectPtr},
    },
    text::TextBaseline,
    widget::{AnyWidget, ChildrenElement, Widget},
};

type OnMount = Box<dyn Fn(&mut UpdateCtx<'_>)>;
type OnUpdate = Box<dyn Fn(&mut UpdateCtx<'_>)>;
type OnMessage = Box<dyn Fn(&mut MessageCtx<'_>)>;
type OnRebuild = Box<dyn Fn(&mut UpdateCtx<'_>)>;

/// Leaf widget whose lifecycle behavior is supplied by closures, so a test can observe when each hook runs.
#[allow(clippy::struct_field_names)]
pub struct Leaf {
    on_mount: OnMount,
    on_update: OnUpdate,
    on_message: OnMessage,
    on_rebuild: OnRebuild,
}

impl Leaf {
    pub fn new() -> Self {
        Self {
            on_mount: Box::new(|_| {}),
            on_update: Box::new(|_| {}),
            on_message: Box::new(|_| {}),
            on_rebuild: Box::new(|_| {}),
        }
    }

    pub fn on_mount(mut self, f: impl Fn(&mut UpdateCtx<'_>) + 'static) -> Self {
        self.on_mount = Box::new(f);
        self
    }

    pub fn on_update(mut self, f: impl Fn(&mut UpdateCtx<'_>) + 'static) -> Self {
        self.on_update = Box::new(f);
        self
    }

    pub fn on_message(mut self, f: impl Fn(&mut MessageCtx<'_>) + 'static) -> Self {
        self.on_message = Box::new(f);
        self
    }

    pub fn on_rebuild(mut self, f: impl Fn(&mut UpdateCtx<'_>) + 'static) -> Self {
        self.on_rebuild = Box::new(f);
        self
    }
}

impl Default for Leaf {
    fn default() -> Self {
        Self::new()
    }
}

/// The element of [`Leaf`], holding the lifecycle closures moved out of the widget. `on_mount` fires from the
/// element's `mount` hook, where a tree handle exists, rather than at create.
pub struct LeafElement {
    on_mount: OnMount,
    on_message: OnMessage,
    on_rebuild: OnRebuild,
    render: (),
}

impl Element for LeafElement {
    type Render = ();

    fn render_object_mut(&mut self) -> &mut () {
        &mut self.render
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<()> {
        RenderObjectPtr::dangling()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        (self.on_mount)(ctx);
    }

    fn rebuild(&mut self, ctx: &mut UpdateCtx<'_>) {
        (self.on_rebuild)(ctx);
    }

    fn dependency_changed(&mut self, ctx: &mut UpdateCtx<'_>) {
        (self.on_rebuild)(ctx);
    }

    fn message(&mut self, ctx: &mut MessageCtx<'_>) {
        (self.on_message)(ctx);
    }
}

impl Widget for Leaf {
    type Element = LeafElement;

    type Render = ();

    fn create(self, _ctx: &mut CreateCtx) -> Self::Element {
        LeafElement {
            on_mount: self.on_mount,
            on_message: self.on_message,
            on_rebuild: self.on_rebuild,
            render: (),
        }
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        (self.on_update)(ctx);

        element.on_mount = self.on_mount;
        element.on_message = self.on_message;
        element.on_rebuild = self.on_rebuild;
    }
}

/// A widget that adds no node of its own, forwarding its element straight to its child.
pub struct Transparent<Child> {
    pub child: Child,
}

impl<Child: Widget> Widget for Transparent<Child> {
    type Element = Child::Element;

    type Render = Child::Render;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        self.child.create(ctx)
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        self.child.update(ctx, element);
    }
}

/// A minimal multi-child render object, holding its children's render objects in order through the uniform
/// `dyn RenderBox` edges a sequence produces.
pub struct MultiChildRenderList {
    pub children: Vec<RenderNode<dyn RenderBox>>,
}

impl MultiChildRenderObject for MultiChildRenderList {
    type Children = Vec<RenderNode<dyn RenderBox>>;

    fn children_mut(&mut self) -> &mut Vec<RenderNode<dyn RenderBox>> {
        &mut self.children
    }
}

impl RenderObject for MultiChildRenderList {}

impl RenderBox for MultiChildRenderList {
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
        for child in &mut self.children {
            child.layout(ctx, constraints);
        }

        constraints.smallest()
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
        HitTest::Pass
    }

    fn update_compositing_bits(&mut self) -> bool {
        let mut needs = false;

        for child in &mut self.children {
            needs |= child.update_compositing_bits();
        }

        needs
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        for child in &mut self.children {
            child.paint(ctx, offset);
        }
    }
}

/// A widget holding a keyed run of children, rendering them through [`MultiChildRenderList`].
pub struct MultiChild<Child> {
    pub children: Vec<Child>,
}

impl<Child> Widget for MultiChild<Child>
where
    Child: Widget + 'static,
    Child::Render: RenderBox + Sized + 'static,
    Child::Element: 'static,
{
    type Element = ChildrenElement<Vec<Child>, MultiChildRenderList>;

    type Render = MultiChildRenderList;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        ChildrenElement::new(ctx, self.children, |children| MultiChildRenderList {
            children,
        })
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        element.update(ctx, self.children);
    }
}

/// Shared observation cells for a [`Probe`]: its lifecycle tallies, the tree handle it captures at mount, and
/// the last message delivered to it. A test holds the log and reads it after driving frames; cloning shares
/// the cells, so the same log given to a probe across frames tracks one element instance.
#[derive(Clone, Default)]
pub struct ProbeLog {
    pub mounts: Rc<Cell<usize>>,
    pub updates: Rc<Cell<usize>>,
    pub unmounts: Rc<Cell<usize>>,
    pub handle: Rc<Cell<Option<NodeHandle>>>,
    pub received: Rc<Cell<Option<u32>>>,
}

/// A keyed leaf fixture for reconcile tests. It records mount, reconcile, and unmount into a shared
/// [`ProbeLog`], captures its handle at mount so a test can address it, and stores the last `u32` message it
/// receives.
pub struct Probe {
    key: u32,
    log: ProbeLog,
}

impl Probe {
    pub fn new(key: u32, log: ProbeLog) -> Self {
        Self { key, log }
    }
}

/// The element of [`Probe`].
pub struct ProbeElement {
    log: ProbeLog,
    render: (),
}

impl Element for ProbeElement {
    type Render = ();

    fn render_object_mut(&mut self) -> &mut () {
        &mut self.render
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<()> {
        RenderObjectPtr::dangling()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.log.mounts.set(self.log.mounts.get() + 1);
        self.log.handle.set(Some(ctx.handle()));
    }

    fn unmount(&mut self, _ctx: &mut UpdateCtx<'_>) {
        self.log.unmounts.set(self.log.unmounts.get() + 1);
    }

    fn message(&mut self, ctx: &mut MessageCtx<'_>) {
        let value = ctx.consume::<u32>();
        self.log.received.set(Some(value));
    }
}

impl Widget for Probe {
    type Element = ProbeElement;

    type Render = ();

    fn create(self, _ctx: &mut CreateCtx) -> Self::Element {
        ProbeElement {
            log: self.log,
            render: (),
        }
    }

    fn update(self, _ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        element.log = self.log;
        element.log.updates.set(element.log.updates.get() + 1);
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        Some(&self.key)
    }
}

/// A leaf box render for frame tests: it sizes to the smallest size the constraints allow, recording the size
/// it was laid out at and how many times it painted, so a test can drive a real layout/paint frame and
/// observe it.
pub struct RecordingBox {
    pub laid_out: Rc<Cell<Option<Size>>>,
    pub layouts: Rc<Cell<usize>>,
    pub paints: Rc<Cell<usize>>,
    /// A handle to this render object's enclosing relayout boundary, captured each layout, so a test can mark
    /// it and drive an isolated re-lay.
    pub boundary: Rc<RefCell<Option<DeferredLayoutScope>>>,
}

impl RecordingBox {
    pub fn new() -> Self {
        Self {
            laid_out: Rc::new(Cell::new(None)),
            layouts: Rc::new(Cell::new(0)),
            paints: Rc::new(Cell::new(0)),
            boundary: Rc::new(RefCell::new(None)),
        }
    }
}

impl Default for RecordingBox {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderObject for RecordingBox {}

impl RenderBox for RecordingBox {
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
        let size = constraints.smallest();
        self.laid_out.set(Some(size));
        self.layouts.set(self.layouts.get() + 1);
        *self.boundary.borrow_mut() = Some(ctx.deferred_layout_scope());
        size
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
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

    fn paint(&mut self, _: &mut PaintCtx, _: Offset) {
        self.paints.set(self.paints.get() + 1);
    }
}

/// The render object of a [`Single`]: a single-child passthrough holding its child by an edge, so a test can
/// exercise the concrete (unwrapped) single-child path.
pub struct RenderSingle<C: ?Sized> {
    child: RenderNode<C>,
}

impl<C: ?Sized> RenderSingle<C> {
    pub fn new() -> Self {
        Self {
            child: RenderNode::new(()),
        }
    }
}

impl<C: ?Sized> Default for RenderSingle<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: ?Sized> SingleChildRenderObject for RenderSingle<C> {
    type Child = C;

    fn adopt_child(&mut self, child: MountedChild<C>) {
        self.child.set(child);
    }
}

/// A single-child widget, reusing [`SingleChildElement`] with a concrete child render edge.
pub struct Single<Child> {
    pub child: Child,
}

impl<Child: Widget> Widget for Single<Child> {
    type Element = SingleChildElement<Child::Element, RenderSingle<Child::Render>>;

    type Render = RenderSingle<Child::Render>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        SingleChildElement::new(ctx, self.child, RenderSingle::new())
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        element.update(ctx, self.child);
    }
}

/// A multi-child widget whose children are type-erased boxed widgets, exercising the runtime-mixed
/// (`Box<dyn AnyWidget>`) sequence path through the same [`MultiChildRenderList`].
pub struct BoxedChildren {
    pub children: Vec<Box<dyn AnyWidget<Render = dyn RenderBox>>>,
}

impl Widget for BoxedChildren {
    type Element =
        ChildrenElement<Vec<Box<dyn AnyWidget<Render = dyn RenderBox>>>, MultiChildRenderList>;

    type Render = MultiChildRenderList;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        ChildrenElement::new(ctx, self.children, |children| MultiChildRenderList {
            children,
        })
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        element.update(ctx, self.children);
    }
}

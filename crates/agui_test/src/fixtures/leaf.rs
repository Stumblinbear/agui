use typed_floats::{Positive, PositiveFinite};

use agui::{
    context::{CreateCtx, LayoutCtx, MessageCtx, PaintCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::{Element, LeafElement as LeafRenderElement},
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    paint::peniko::{Color, Fill},
    render_object::{
        RenderObject,
        box_layout::{BoxConstraints, RenderBox},
        node::RenderObjectPtr,
    },
    semantics::SemanticsTreeBuilder,
    text::TextBaseline,
    widget::Widget,
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

/// The element of [`Leaf`], holding the lifecycle closures moved out of the widget. `on_mount` fires
/// from the element's `mount` hook, where a tree handle exists, rather than at create.
#[allow(clippy::struct_field_names)]
pub struct LeafElement {
    on_mount: OnMount,
    on_message: OnMessage,
    on_rebuild: OnRebuild,
}

// SAFETY: a leaf with no children; its `()` render object is never dereferenced.
unsafe impl Element for LeafElement {
    type Render = ();

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

/// A leaf widget of a fixed size, optionally filling itself with a color.
///
/// It takes the size it was given regardless of the constraints it is laid out under, so a test can
/// place content of a known size under a layout widget and assert on the result. Give it a color to
/// have it paint a fill the size of its bounds, which a test can then find in the composited scene.
pub struct TestBox {
    size: Size,
    color: Option<Color>,
}

impl TestBox {
    pub fn new(size: Size) -> Self {
        Self { size, color: None }
    }

    /// Paints a fill of `color` covering the box's bounds.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl Widget for TestBox {
    type Element = LeafRenderElement<RenderTestBox>;

    type Render = RenderTestBox;

    fn create(self, _: &mut CreateCtx) -> LeafRenderElement<RenderTestBox> {
        LeafRenderElement::new(RenderTestBox {
            size: self.size,
            color: self.color,
        })
    }

    fn update(self, _: &mut UpdateCtx, element: &mut LeafRenderElement<RenderTestBox>) {
        let render_object = element.render_object_mut();
        render_object.size = self.size;
        render_object.color = self.color;
    }
}

/// The render object of a [`TestBox`].
pub struct RenderTestBox {
    size: Size,
    color: Option<Color>,
}

impl RenderObject for RenderTestBox {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>().finish()
    }
}

impl RenderBox for RenderTestBox {
    fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.size.width.get()).ok()
    }

    fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.size.width.get()).ok()
    }

    fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.size.height.get()).ok()
    }

    fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.size.height.get()).ok()
    }

    fn measure(&self, _: BoxConstraints) -> Size {
        self.size
    }

    fn layout(&mut self, _: &mut LayoutCtx, _: BoxConstraints) -> Size {
        self.size
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, _: &mut HitTestResult, position: Offset) -> HitTest {
        let x = position.x.get();
        let y = position.y.get();

        if x >= 0.0 && y >= 0.0 && x <= self.size.width.get() && y <= self.size.height.get() {
            HitTest::Absorb
        } else {
            HitTest::Pass
        }
    }

    fn update_compositing_bits(&mut self) -> bool {
        false
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        if let Some(color) = self.color {
            let mut canvas = ctx.canvas();
            let brush = canvas.brush(color);
            canvas.fill(Fill::NonZero, brush, &(offset & self.size));
        }
    }

    fn build_semantics(&mut self, _: &mut SemanticsTreeBuilder<'_>) {}
}

/// A leaf with independently chosen intrinsic sizes, for use as a child whose metrics a test controls.
///
/// A box that has children usually derives its own intrinsics from theirs, so put one of these under
/// the box being tested and the box's intrinsics become predictable. It lays out to its given size
/// clamped to the constraints, and reports that size as both its minimum and maximum intrinsic unless
/// [`min_intrinsic`](Self::min_intrinsic) or [`max_intrinsic`](Self::max_intrinsic) sets them apart,
/// which lets a test confirm a parent threads the minimum and maximum through to the right places.
pub struct IntrinsicBox {
    size: Size,
    min: Size,
    max: Size,
}

impl IntrinsicBox {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            min: size,
            max: size,
        }
    }

    /// Sets the minimum intrinsic width and height the box reports.
    pub fn min_intrinsic(mut self, min: Size) -> Self {
        self.min = min;
        self
    }

    /// Sets the maximum intrinsic width and height the box reports.
    pub fn max_intrinsic(mut self, max: Size) -> Self {
        self.max = max;
        self
    }
}

impl Widget for IntrinsicBox {
    type Element = LeafRenderElement<RenderIntrinsicBox>;

    type Render = RenderIntrinsicBox;

    fn create(self, _: &mut CreateCtx) -> LeafRenderElement<RenderIntrinsicBox> {
        LeafRenderElement::new(RenderIntrinsicBox {
            size: self.size,
            min: self.min,
            max: self.max,
        })
    }

    fn update(self, _: &mut UpdateCtx, element: &mut LeafRenderElement<RenderIntrinsicBox>) {
        let render_object = element.render_object_mut();
        render_object.size = self.size;
        render_object.min = self.min;
        render_object.max = self.max;
    }
}

/// The render object of an [`IntrinsicBox`].
pub struct RenderIntrinsicBox {
    size: Size,
    min: Size,
    max: Size,
}

impl RenderObject for RenderIntrinsicBox {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>().finish()
    }
}

impl RenderBox for RenderIntrinsicBox {
    fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.min.width.get()).ok()
    }

    fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.max.width.get()).ok()
    }

    fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.min.height.get()).ok()
    }

    fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.max.height.get()).ok()
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        constraints.constrain(self.size)
    }

    fn layout(&mut self, _: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        constraints.constrain(self.size)
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

    fn paint(&mut self, _: &mut PaintCtx, _: Offset) {}

    fn build_semantics(&mut self, _: &mut SemanticsTreeBuilder<'_>) {}
}

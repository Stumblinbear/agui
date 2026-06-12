use typed_floats::{Positive, PositiveFinite};

use crate::{
    context::{Dispatch, LayoutCtx, MessageCtx, MountCtx, PaintCtx, UpdateCtx},
    element::{Element, MultiChildElement, RoutingId},
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    render_object::{
        MultiChildRenderObject, RenderObject,
        box_layout::{BoxConstraints, RenderBox},
        node::RenderNode,
    },
    text::TextBaseline,
    widget::Widget,
};

type OnMount = Box<dyn Fn(&mut UpdateCtx<'_>)>;
type OnUpdate = Box<dyn Fn(&mut UpdateCtx<'_>)>;
type OnMessage = Box<dyn Fn(&mut MessageCtx)>;
type OnRebuild = Box<dyn Fn(&mut UpdateCtx<'_>)>;

/// Leaf widget whose lifecycle behavior is supplied by closures.
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

    pub fn on_message(mut self, f: impl Fn(&mut MessageCtx) + 'static) -> Self {
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

/// The element of [`Leaf`], holding the dispatch closures moved out of the widget.
pub struct LeafElement {
    on_message: OnMessage,
    on_rebuild: OnRebuild,
}

impl Element for LeafElement {
    type Render = ();

    fn dispatch(&mut self, (): &mut (), path: &[RoutingId], action: Dispatch) {
        debug_assert!(path.is_empty(), "Leaf has no children");

        if !path.is_empty() {
            return;
        }

        match action {
            Dispatch::Message(ctx) => (self.on_message)(ctx),
            Dispatch::Rebuild(ctx) | Dispatch::DependencyChanged(ctx) => (self.on_rebuild)(ctx),
        }
    }
}

impl Widget for Leaf {
    type Element = LeafElement;

    type Render = ();

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        (self.on_mount)(ctx);

        (
            LeafElement {
                on_message: self.on_message,
                on_rebuild: self.on_rebuild,
            },
            (),
        )
    }

    fn update(self, element: &mut Self::Element, (): &mut Self::Render, ctx: &mut UpdateCtx) {
        (self.on_update)(ctx);

        element.on_message = self.on_message;
        element.on_rebuild = self.on_rebuild;
    }
}

pub struct Transparent<Child> {
    pub child: Child,
}

impl<Child: Widget> Widget for Transparent<Child> {
    type Element = Child::Element;

    type Render = Child::Render;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        self.child.create(ctx)
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        self.child.update(element, render_object, ctx);
    }
}

/// A minimal [`MultiChildRender`] container, holding its children's render objects in order.
pub struct MultiChildRenderList<C> {
    pub children: Vec<RenderNode<C>>,
}

impl<C> MultiChildRenderObject for MultiChildRenderList<C> {
    type Child = C;

    fn take_children(&mut self) -> Vec<RenderNode<C>> {
        std::mem::take(&mut self.children)
    }

    fn set_children(&mut self, children: Vec<RenderNode<C>>) {
        self.children = children;
    }

    fn with_child<R>(&self, index: usize, f: impl FnOnce(&C) -> R) -> R {
        f(&self.children[index].object)
    }

    fn with_child_mut<R>(&mut self, index: usize, f: impl FnOnce(&mut C) -> R) -> R {
        f(&mut self.children[index].object)
    }
}

impl<C: RenderBox> RenderBox for MultiChildRenderList<C> {
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

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        for child in &mut self.children {
            child.paint(ctx, offset);
        }
    }
}

impl<C: RenderObject> RenderObject for MultiChildRenderList<C> {
    fn mount(&mut self, ctx: &mut MountCtx) {
        for child in &mut self.children {
            child.mount(ctx);
        }
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        for child in &mut self.children {
            child.unmount(ctx);
        }
    }

    fn update_compositing_bits(&mut self) -> bool {
        let mut needs = false;
        for child in &mut self.children {
            needs |= child.update_compositing_bits();
        }
        needs
    }
}

pub struct MultiChild<Child> {
    pub children: Vec<Child>,
}

impl<Child> Widget for MultiChild<Child>
where
    Child: Widget + 'static,
    Child::Render: RenderObject,
{
    type Element = MultiChildElement<Child::Element, MultiChildRenderList<Child::Render>>;

    type Render = MultiChildRenderList<Child::Render>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let mut render = MultiChildRenderList {
            children: Vec::new(),
        };

        let element = MultiChildElement::new(self.children, &mut render, ctx);

        (element, render)
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        element.update(self.children, render_object, ctx);
    }
}

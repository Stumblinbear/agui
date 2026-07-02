use std::{cell::Cell, rc::Rc};

use typed_floats::{Positive, PositiveFinite};

use agui_core::tree::NodeHandle;

use agui::{
    context::{CreateCtx, LayoutCtx, MessageCtx, PaintCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::Element,
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    key::AnyKeyable,
    pipeline::render_pipeline::LayoutScope,
    render_object::{
        MultiChildRenderObject, RenderObject,
        box_layout::{BoxConstraints, RenderBox},
        node::{RenderNode, RenderObjectPtr},
    },
    semantics::SemanticsTreeBuilder,
    text::TextBaseline,
    widget::{AnyWidget, ChildrenElement, Widget},
};

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

    // Detached: this fixture backs reconcile tests, which do not assert on an incremental-grow re-layout.
    fn layout_scope(&self) -> LayoutScope {
        LayoutScope::detached()
    }
}

impl RenderObject for MultiChildRenderList {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.children
            .iter()
            .fold(d.node_for::<Self>(), |node, child| {
                node.child(|d| child.describe(d))
            })
            .finish()
    }
}

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

    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        for child in &mut self.children {
            child.build_semantics(s);
        }
    }
}

/// Shared observation cells for a [`KeyedLeaf`]: its lifecycle tallies, the tree handle it captures at
/// mount, and the last message delivered to it. A test holds the log and reads it after driving frames;
/// cloning shares the cells, so the same log given to a probe across frames tracks one element instance.
#[derive(Clone, Default)]
pub struct ProbeLog {
    pub mounts: Rc<Cell<usize>>,
    pub updates: Rc<Cell<usize>>,
    pub unmounts: Rc<Cell<usize>>,
    pub handle: Rc<Cell<Option<NodeHandle>>>,
    pub received: Rc<Cell<Option<u32>>>,
}

/// A keyed leaf fixture for reconcile tests. It records mount, reconcile, and unmount into a shared
/// [`ProbeLog`], captures its handle at mount so a test can address it, and stores the last `u32` message
/// it receives.
pub struct KeyedLeaf {
    key: u32,
    log: ProbeLog,
}

impl KeyedLeaf {
    pub fn new(key: u32, log: ProbeLog) -> Self {
        Self { key, log }
    }
}

/// The element of [`KeyedLeaf`].
pub struct KeyedLeafElement {
    log: ProbeLog,
}

// SAFETY: a leaf with no children; its `()` render object is never dereferenced.
unsafe impl Element for KeyedLeafElement {
    type Render = ();

    fn render_object_ptr(&self) -> RenderObjectPtr<()> {
        RenderObjectPtr::unit()
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

impl Widget for KeyedLeaf {
    type Element = KeyedLeafElement;

    type Render = ();

    fn create(self, _ctx: &mut CreateCtx) -> Self::Element {
        KeyedLeafElement { log: self.log }
    }

    fn update(self, _ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        element.log = self.log;
        element.log.updates.set(element.log.updates.get() + 1);
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        Some(&self.key)
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

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use slotmap::Key;

use crate::{
    context::PaintCtx,
    geometry::Offset,
    paint::{
        compositing::{LayerHandle, OffsetLayer},
        scene::SceneCapacity,
    },
    pipeline::{BoundaryContent, FramePhase, enter_phase},
    reactor::{Node, NodeId, Reaction, Reactor},
    render_object::box_layout::RenderBox,
};

/// Owns the repaint boundaries of one subtree and repaints the ones that have changed.
///
/// The subtree's root paints into the layer composited for presentation; every boundary nested inside
/// paints into its own retained layer and can be repainted on its own, leaving every other boundary's
/// layer untouched. Mark the root through the [`PaintBoundaryHandle`] returned when the pipeline is
/// built, and an inner boundary through the one handed back when it is registered.
pub struct PaintPipeline {
    inner: Rc<RefCell<PaintInner>>,
}

/// The out-of-band marks queued by [`DeferredPaintScope`]s, shared so a deferred marker can push
/// without the pipeline in hand.
type DeferredQueue = Rc<RefCell<Vec<(NodeId, PaintPhase)>>>;

/// The reactor holding the boundaries, plus the recomposite flag. A boundary owes its repaint and
/// compositing-bits work to the reactor. A bare recomposite names no boundary to repaint, so it stays a
/// flag here on its own.
struct PaintInner {
    reactor: Reactor<PaintNode, 2>,

    /// Whether a layer's placement changed and the subtree must recomposite, with no boundary to
    /// repaint. Cleared when the frame composites.
    needs_composite: bool,

    deferred: DeferredQueue,

    /// Reused across flushes to hold one phase's drained ids, so its capacity survives between frames.
    scratch: Vec<NodeId>,

    /// Fired when the pipeline goes from no pending work to some, so the driver schedules a frame.
    notify: Box<dyn Fn()>,
}

/// What a boundary owes, which are also the flush phases. Bits settle before paint, so a boundary marked
/// for both recomputes its compositing bits and then repaints against them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PaintPhase {
    CompositingBits,
    Paint,
    Composite,
}

impl Reaction for PaintPhase {
    fn index(self) -> u8 {
        match self {
            Self::CompositingBits => 0,
            Self::Paint => 1,
            Self::Composite => 2,
        }
    }
}

/// A repaint boundary as the reactor sees it: opaque content painting into a retained layer.
struct PaintNode {
    /// The depth in the boundary nesting, so a flush re-enters rootmost-first.
    depth: usize,

    content: BoundaryContent,
    layer: LayerHandle<OffsetLayer>,

    /// The buffer lengths the last repaint recorded, sizing the next repaint's buffers.
    paint_capacity: SceneCapacity,
}

impl Node for PaintNode {
    type Reaction = PaintPhase;

    fn depth(&self) -> usize {
        self.depth
    }
}

impl Default for PaintPipeline {
    fn default() -> Self {
        Self {
            inner: Rc::new(RefCell::new(PaintInner {
                reactor: Reactor::default(),
                needs_composite: false,
                deferred: Rc::new(RefCell::new(Vec::new())),
                scratch: Vec::new(),
                notify: Box::new(|| {}),
            })),
        }
    }
}

impl PaintPipeline {
    /// Registers `root` as a boundary painting into `layer` and returns the pipeline together with the
    /// handle that owns it.
    pub fn new(
        root: BoundaryContent,
        layer: LayerHandle<OffsetLayer>,
    ) -> (Self, PaintBoundaryHandle) {
        let mut pipeline = Self::default();
        let handle = pipeline.register_boundary(PaintScope::detached(), root, layer);

        (pipeline, handle)
    }

    pub fn on_needs_paint(&mut self, f: Box<dyn Fn()>) {
        self.inner.borrow_mut().notify = f;
    }

    /// Adds `content` as a boundary nested under `enclosing`, painting into `layer`, and returns the
    /// [`PaintBoundaryHandle`] that owns it and hands out [`PaintScope`]s for marking it. Its depth is one
    /// past the enclosing boundary's, so a flush re-enters it after the boundary that may repaint over it.
    /// A detached or removed enclosing scope registers at depth zero, the pipeline's root.
    pub fn register_boundary(
        &mut self,
        enclosing: PaintScope,
        content: BoundaryContent,
        layer: LayerHandle<OffsetLayer>,
    ) -> PaintBoundaryHandle {
        let depth = match self.inner.borrow().reactor.get(enclosing.0) {
            Some(node) => node.depth + 1,
            None => 0,
        };

        let node_id = {
            let mut inner = self.inner.borrow_mut();

            let was_clean = inner.is_clean();

            let node_id = inner.reactor.register(PaintNode {
                depth,
                content,
                layer,
                paint_capacity: SceneCapacity::default(),
            });

            // A fresh boundary owes a compositing-bits settle and a first paint.
            inner.reactor.mark(node_id, PaintPhase::CompositingBits);
            inner.reactor.mark(node_id, PaintPhase::Paint);

            if was_clean {
                (inner.notify)();
            }

            node_id
        };

        tracing::debug!(boundary = ?node_id, "registered repaint boundary");

        PaintBoundaryHandle {
            node_id,
            inner: Rc::downgrade(&self.inner),
        }
    }

    /// Removes a boundary, discarding its layer and clearing any pending mark. The handle is spent.
    // Taking the handle by value spends it, so it cannot mark a boundary that no longer exists.
    #[allow(clippy::needless_pass_by_value)]
    pub fn unregister(&mut self, handle: PaintBoundaryHandle) {
        drop(handle);
    }

    /// Marks `scope`'s boundary to be repainted on the next frame.
    pub fn mark_needs_paint(&self, scope: PaintScope) {
        self.inner.borrow_mut().mark_needs_paint(scope.0);
    }

    /// Marks `scope`'s compositing bits for recomputation before its next repaint, and the boundary for
    /// repaint.
    pub fn mark_needs_compositing_bits_update(&self, scope: PaintScope) {
        self.inner
            .borrow_mut()
            .mark_needs_compositing_bits_update(scope.0);
    }

    /// Schedules a recomposite of the subtree on the next frame, without repainting any boundary.
    pub fn mark_needs_composite(&self, _scope: PaintScope) {
        self.inner.borrow_mut().mark_needs_composite();
    }

    /// A deferred handle to `scope`'s boundary, for marking it from a callback that runs with no
    /// pipeline in hand, such as a per-frame animation.
    pub fn deferred_scope(&self, scope: PaintScope) -> DeferredPaintScope {
        DeferredPaintScope {
            node_id: scope.0,
            queue: Some(Rc::clone(&self.inner.borrow().deferred)),
        }
    }

    /// Applies every out-of-band mark queued since the last drain. Called once at the start of a frame,
    /// before the channels are flushed.
    pub fn drain_deferred(&mut self) {
        let queue = Rc::clone(&self.inner.borrow().deferred);
        let drained: Vec<(NodeId, PaintPhase)> = queue.borrow_mut().drain(..).collect();

        let mut inner = self.inner.borrow_mut();

        for (node_id, kind) in drained {
            match kind {
                PaintPhase::Paint => inner.mark_needs_paint(node_id),
                PaintPhase::CompositingBits => inner.mark_needs_compositing_bits_update(node_id),
                PaintPhase::Composite => inner.mark_needs_composite(),
            }
        }
    }

    /// Recomposites the subtree.
    pub fn flush(&mut self) {
        let mut scratch = {
            let mut inner = self.inner.borrow_mut();
            std::mem::take(&mut inner.scratch)
        };

        self.flush_compositing_bits(&mut scratch);
        self.flush_paint(&mut scratch);
        self.flush_composite(&mut scratch);

        self.inner.borrow_mut().scratch = scratch;
    }

    /// Recomputes the compositing bits of every marked boundary.
    fn flush_compositing_bits(&mut self, scratch: &mut Vec<NodeId>) {
        let _phase = enter_phase(FramePhase::CompositingBits);

        self.inner
            .borrow_mut()
            .reactor
            .take_dirty(PaintPhase::CompositingBits, scratch);

        for &node_id in &*scratch {
            let mut content = {
                match self.inner.borrow().reactor.get(node_id) {
                    Some(node) => Rc::clone(&node.content),
                    None => continue,
                }
            };

            content.update_compositing_bits();
        }
    }

    /// Repaints every marked boundary.
    fn flush_paint(&mut self, scratch: &mut Vec<NodeId>) {
        let _phase = enter_phase(FramePhase::Paint);

        self.inner
            .borrow_mut()
            .reactor
            .take_dirty(PaintPhase::Paint, scratch);

        for &node_id in &*scratch {
            let (mut content, layer, capacity) = {
                match self.inner.borrow().reactor.get(node_id) {
                    Some(node) => (
                        Rc::clone(&node.content),
                        node.layer.clone(),
                        node.paint_capacity,
                    ),
                    None => continue,
                }
            };

            layer.borrow_mut().clear();

            let recorded = PaintCtx::paint_with_capacity(&layer, capacity, |ctx| {
                content.paint(ctx, Offset::ZERO);
            });

            if let Some(node) = self.inner.borrow_mut().reactor.get_mut(node_id) {
                node.paint_capacity = recorded;
            }
        }
    }

    /// Recomposites the subtree.
    fn flush_composite(&mut self, _: &mut Vec<NodeId>) {
        let _phase = enter_phase(FramePhase::Composite);

        let mut inner = self.inner.borrow_mut();
        inner.needs_composite = false;
    }
}

impl PaintInner {
    fn mark_needs_paint(&mut self, node_id: NodeId) {
        FramePhase::assert_can_mark(FramePhase::Paint);

        tracing::trace!(boundary = ?node_id, "marked boundary for repaint");

        let was_clean = self.is_clean();
        self.reactor.mark(node_id, PaintPhase::Paint);

        if was_clean {
            (self.notify)();
        }
    }

    fn mark_needs_compositing_bits_update(&mut self, node_id: NodeId) {
        FramePhase::assert_can_mark(FramePhase::CompositingBits);

        tracing::trace!(boundary = ?node_id, "marked boundary for compositing bits update");

        let was_clean = self.is_clean();

        // A bits change alters how the boundary paints, so it owes both phases.
        self.reactor.mark(node_id, PaintPhase::CompositingBits);
        self.reactor.mark(node_id, PaintPhase::Paint);

        if was_clean {
            (self.notify)();
        }
    }

    fn mark_needs_composite(&mut self) {
        FramePhase::assert_can_mark(FramePhase::Composite);

        let was_clean = self.is_clean();

        self.needs_composite = true;

        if was_clean {
            (self.notify)();
        }
    }

    /// Whether neither the boundaries nor the standalone recomposite has pending work.
    fn is_clean(&self) -> bool {
        self.reactor.is_clean() && !self.needs_composite
    }
}

/// Names the repaint boundary a render object paints into. A node holds the scope of its nearest
/// enclosing boundary and presents it to a context, or a [`DeferredPaintScope`], to repaint that
/// boundary when its painting goes stale.
#[derive(Clone, Copy)]
pub struct PaintScope(NodeId);

impl PaintScope {
    /// A scope detached from any pipeline, which names no boundary.
    pub fn detached() -> Self {
        Self(NodeId::null())
    }

    /// Whether this scope names no boundary, because it was created detached.
    pub fn is_detached(&self) -> bool {
        self.0.is_null()
    }
}

/// A [`PaintScope`] paired with the route to mark it from outside a pipeline pass, such as a per-frame
/// animation callback that holds no context. Each request is applied on the pipeline's next frame; a
/// detached marker, or one whose boundary is gone, marks nothing.
#[derive(Clone)]
pub struct DeferredPaintScope {
    node_id: NodeId,
    queue: Option<DeferredQueue>,
}

impl DeferredPaintScope {
    /// A marker detached from any pipeline, whose marks reach nothing.
    pub fn detached() -> Self {
        Self {
            node_id: NodeId::null(),
            queue: None,
        }
    }

    /// Queues this boundary to be repainted on the pipeline's next frame.
    pub fn mark_needs_paint(&self) {
        self.push(PaintPhase::Paint);
    }

    /// Queues this boundary's compositing bits to be recomputed, and the boundary repainted, on the
    /// next frame.
    pub fn mark_needs_compositing_bits_update(&self) {
        self.push(PaintPhase::CompositingBits);
    }

    /// Queues a recomposite of the subtree for the next frame, without repainting this boundary.
    pub fn mark_needs_composite(&self) {
        self.push(PaintPhase::Composite);
    }

    fn push(&self, kind: PaintPhase) {
        if let Some(queue) = &self.queue {
            queue.borrow_mut().push((self.node_id, kind));
        }
    }
}

/// A registered boundary, returned to the render object that registered it. It owns the boundary's
/// place in the pipeline and is the only thing [`unregister`](PaintPipeline::unregister) accepts; it
/// hands out mark-only [`PaintScope`]s for the subtree. Keeping removal here, off the scope, stops a
/// descendant that was handed a scope to mark with from unregistering the boundary it lives under.
pub struct PaintBoundaryHandle {
    node_id: NodeId,
    inner: Weak<RefCell<PaintInner>>,
}

impl PaintBoundaryHandle {
    /// A mark-only handle to this boundary, for descendants to repaint into it.
    pub fn scope(&self) -> PaintScope {
        PaintScope(self.node_id)
    }

    /// Marks this boundary to be repainted on the next frame.
    pub fn mark_needs_paint(&self) {
        if let Some(inner) = self.inner.upgrade() {
            inner.borrow_mut().mark_needs_paint(self.node_id);
        }
    }

    /// Marks this boundary's compositing bits for recomputation before its next repaint.
    pub fn mark_needs_compositing_bits_update(&self) {
        if let Some(inner) = self.inner.upgrade() {
            inner
                .borrow_mut()
                .mark_needs_compositing_bits_update(self.node_id);
        }
    }

    /// Schedules a recomposite of the subtree on the next frame, without repainting this boundary.
    pub fn mark_needs_composite(&self) {
        if let Some(inner) = self.inner.upgrade() {
            inner.borrow_mut().mark_needs_composite();
        }
    }
}

impl Drop for PaintBoundaryHandle {
    fn drop(&mut self) {
        let Some(inner) = self.inner.upgrade() else {
            return;
        };

        // The removed node owns the boundary's render content, whose own drop can unregister a nested
        // boundary and so re-enter this borrow. Hold the content until the borrow is released, then let
        // it drop. Removing it makes the boundary absent: a flush that already drained its id skips it.
        let removed = {
            let mut inner = inner.borrow_mut();
            inner
                .deferred
                .borrow_mut()
                .retain(|(queued, _)| *queued != self.node_id);
            inner.reactor.remove(self.node_id)
        };

        drop(removed);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        time::Duration,
    };

    use typed_floats::{Positive, PositiveFinite, as_const};

    use crate::{
        context::{LayoutCtx, MountCtx},
        geometry::{Offset, Size},
        input::hit_test::{HitTest, HitTestResult},
        paint::{
            command::PaintCommand,
            compositing::Compositor,
            peniko::{Color, Fill},
            scene::Scene,
        },
        render_object::{
            RenderObject,
            box_layout::{AnyRenderBox, BoxConstraints, RenderBox},
        },
        scheduling::Vsync,
        text::TextBaseline,
    };

    use super::*;

    /// A leaf that counts its paints and fills a unit square, so a test can tell whether it repainted.
    struct Counter {
        paints: Rc<Cell<usize>>,
        color: Color,
    }

    /// A boundary content that fills, then embeds the layers of nested boundaries, standing in for a
    /// render object that hosts child repaint boundaries.
    struct Embedder {
        paints: Rc<Cell<usize>>,
        color: Color,
        children: Vec<LayerHandle<OffsetLayer>>,
    }

    macro_rules! trivial_box_layout {
        () => {
            fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
                Some(as_const!(PositiveFinite, f32, 0.0))
            }

            fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
                Some(as_const!(PositiveFinite, f32, 0.0))
            }

            fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
                Some(as_const!(PositiveFinite, f32, 0.0))
            }

            fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
                Some(as_const!(PositiveFinite, f32, 0.0))
            }

            fn measure(&self, _: BoxConstraints) -> Size {
                Size::new(1.0, 1.0)
            }

            fn layout(&mut self, _: &mut LayoutCtx, _: BoxConstraints) -> Size {
                Size::new(1.0, 1.0)
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
        };
    }

    impl RenderObject for Counter {
        fn mount(&mut self, _: &mut MountCtx) {}
        fn unmount(&mut self, _: &mut MountCtx) {}
    }

    impl RenderBox for Counter {
        trivial_box_layout!();

        fn update_compositing_bits(&mut self) -> bool {
            false
        }

        fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
            self.paints.set(self.paints.get() + 1);

            let mut canvas = ctx.canvas();
            let brush = canvas.brush(self.color);
            canvas.fill(Fill::NonZero, brush, &(offset & Size::new(1.0, 1.0)));
        }
    }

    impl RenderObject for Embedder {
        fn mount(&mut self, _: &mut MountCtx) {}
        fn unmount(&mut self, _: &mut MountCtx) {}
    }

    impl RenderBox for Embedder {
        trivial_box_layout!();

        fn update_compositing_bits(&mut self) -> bool {
            false
        }

        fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
            self.paints.set(self.paints.get() + 1);

            {
                let mut canvas = ctx.canvas();
                let brush = canvas.brush(self.color);
                canvas.fill(Fill::NonZero, brush, &(offset & Size::new(1.0, 1.0)));
            }

            for child in &self.children {
                ctx.add_layer(child.clone(), offset);
            }
        }
    }

    struct Probe {
        paints: Rc<Cell<usize>>,
        bits: Rc<Cell<usize>>,
    }

    impl RenderObject for Probe {
        fn mount(&mut self, _: &mut MountCtx) {}
        fn unmount(&mut self, _: &mut MountCtx) {}
    }

    impl RenderBox for Probe {
        trivial_box_layout!();

        fn update_compositing_bits(&mut self) -> bool {
            self.bits.set(self.bits.get() + 1);
            false
        }

        fn paint(&mut self, _: &mut PaintCtx, _: Offset) {
            self.paints.set(self.paints.get() + 1);
        }
    }

    fn content(render: impl AnyRenderBox + 'static) -> BoundaryContent {
        Rc::new(RefCell::new(render))
    }

    fn layer() -> LayerHandle<OffsetLayer> {
        LayerHandle::new(OffsetLayer::new())
    }

    fn fills(scene: &Scene) -> usize {
        scene
            .flatten()
            .commands()
            .iter()
            .filter(|command| matches!(command, PaintCommand::Fill { .. }))
            .count()
    }

    /// Drains the deferred marks and flushes, as a frame does, against a freestanding pipeline.
    fn flush_paint(pipeline: &mut PaintPipeline) {
        pipeline.drain_deferred();
        pipeline.flush();
    }

    /// A scope sits on every node that paints into a boundary, so it stays a bare id with no pointer or
    /// refcount.
    #[test]
    fn a_paint_scope_is_pointer_free() {
        assert_eq!(
            std::mem::size_of::<PaintScope>(),
            std::mem::size_of::<u64>()
        );
    }

    #[test]
    fn marking_a_boundary_repaints_only_it() {
        let root_paints = Rc::new(Cell::new(0));
        let child_paints = Rc::new(Cell::new(0));

        let root_layer = layer();
        let child_layer = layer();
        let (mut pipeline, root) = PaintPipeline::new(
            content(Embedder {
                paints: Rc::clone(&root_paints),
                color: Color::BLACK,
                children: vec![child_layer.clone()],
            }),
            root_layer.clone(),
        );
        let child = pipeline.register_boundary(
            root.scope(),
            content(Counter {
                paints: Rc::clone(&child_paints),
                color: Color::from_rgb8(255, 0, 0),
            }),
            child_layer,
        );

        flush_paint(&mut pipeline);
        let first = Compositor::compose(&root_layer).rasterize();
        assert_eq!(root_paints.get(), 1);
        assert_eq!(child_paints.get(), 1);
        assert_eq!(
            fills(&first),
            2,
            "the root and child both contributed a fill"
        );

        // The kind of out-of-band mark a per-frame animation callback would make.
        child.mark_needs_paint();

        flush_paint(&mut pipeline);
        let second = Compositor::compose(&root_layer).rasterize();
        assert_eq!(child_paints.get(), 2, "the marked boundary repainted");
        assert_eq!(
            root_paints.get(),
            1,
            "the clean parent boundary was not repainted"
        );
        assert_eq!(
            fills(&second),
            2,
            "the parent still embeds the child through its retained layer"
        );
    }

    #[test]
    fn a_clean_frame_repaints_nothing() {
        let paints = Rc::new(Cell::new(0));

        let (mut pipeline, _root) = PaintPipeline::new(
            content(Counter {
                paints: Rc::clone(&paints),
                color: Color::BLACK,
            }),
            layer(),
        );

        flush_paint(&mut pipeline);
        flush_paint(&mut pipeline);

        assert_eq!(
            paints.get(),
            1,
            "an unmarked boundary paints once and is reused"
        );
    }

    #[test]
    fn marking_schedules_a_frame_on_the_clean_to_dirty_edge() {
        let frames = Rc::new(Cell::new(0));
        let scheduled = Rc::clone(&frames);
        let (mut pipeline, scope) = PaintPipeline::new(
            content(Counter {
                paints: Rc::new(Cell::new(0)),
                color: Color::BLACK,
            }),
            layer(),
        );
        pipeline.on_needs_paint(Box::new(move || scheduled.set(scheduled.get() + 1)));

        // Registration left the boundary dirty; the first frame clears it without involving the hook.
        flush_paint(&mut pipeline);
        assert_eq!(frames.get(), 0, "registration alone schedules no frame");

        scope.mark_needs_paint();
        scope.mark_needs_paint();
        assert_eq!(
            frames.get(),
            1,
            "only the clean-to-dirty edge schedules a frame"
        );

        flush_paint(&mut pipeline);
        scope.mark_needs_paint();
        assert_eq!(
            frames.get(),
            2,
            "a mark after the flush schedules another frame"
        );
    }

    #[test]
    fn a_compositing_bits_mark_also_repaints_but_a_paint_mark_leaves_bits_alone() {
        let paints = Rc::new(Cell::new(0));
        let bits = Rc::new(Cell::new(0));

        let (mut pipeline, boundary) = PaintPipeline::new(
            content(Probe {
                paints: Rc::clone(&paints),
                bits: Rc::clone(&bits),
            }),
            layer(),
        );

        flush_paint(&mut pipeline);
        assert_eq!(paints.get(), 1);
        assert_eq!(
            bits.get(),
            1,
            "registration computes the bits for the first frame"
        );

        boundary.mark_needs_paint();
        flush_paint(&mut pipeline);
        assert_eq!(paints.get(), 2, "the boundary repainted");
        assert_eq!(bits.get(), 1, "a paint mark does not recompute bits");

        boundary.mark_needs_compositing_bits_update();
        flush_paint(&mut pipeline);
        assert_eq!(paints.get(), 3, "the boundary repainted");
        assert_eq!(bits.get(), 2, "a compositing-bits mark recomputes bits");
    }

    #[test]
    fn a_composite_mark_schedules_a_frame_without_repainting() {
        let frames = Rc::new(Cell::new(0));
        let scheduled = Rc::clone(&frames);
        let paints = Rc::new(Cell::new(0));

        let (mut pipeline, boundary) = PaintPipeline::new(
            content(Counter {
                paints: Rc::clone(&paints),
                color: Color::BLACK,
            }),
            layer(),
        );
        pipeline.on_needs_paint(Box::new(move || scheduled.set(scheduled.get() + 1)));

        // Registration left the boundary dirty; the first frame clears it without involving the hook.
        flush_paint(&mut pipeline);
        assert_eq!(paints.get(), 1);
        assert_eq!(frames.get(), 0, "registration alone schedules no frame");

        boundary.mark_needs_composite();
        boundary.mark_needs_composite();
        assert_eq!(
            frames.get(),
            1,
            "only the clean-to-dirty edge schedules a frame"
        );

        flush_paint(&mut pipeline);
        assert_eq!(
            paints.get(),
            1,
            "a composite mark recomposites without repainting the boundary"
        );

        boundary.mark_needs_composite();
        assert_eq!(
            frames.get(),
            2,
            "a mark after the flush schedules another frame"
        );
    }

    #[test]
    fn an_animation_repaints_only_its_own_boundary() {
        let vsync = Vsync::new();

        let static_paints = Rc::new(Cell::new(0));
        let animated_paints = Rc::new(Cell::new(0));

        let static_layer = layer();
        let animated_layer = layer();
        let (mut pipeline, root) = PaintPipeline::new(
            content(Embedder {
                paints: Rc::new(Cell::new(0)),
                color: Color::WHITE,
                children: vec![static_layer.clone(), animated_layer.clone()],
            }),
            layer(),
        );
        // Hold the static boundary's handle: it owns the registration, so dropping it would unregister.
        let _static = pipeline.register_boundary(
            root.scope(),
            content(Counter {
                paints: Rc::clone(&static_paints),
                color: Color::BLACK,
            }),
            static_layer,
        );
        let animated = pipeline.register_boundary(
            root.scope(),
            content(Counter {
                paints: Rc::clone(&animated_paints),
                color: Color::from_rgb8(255, 0, 0),
            }),
            animated_layer,
        );

        flush_paint(&mut pipeline);
        assert_eq!(static_paints.get(), 1);
        assert_eq!(animated_paints.get(), 1);

        // The animation marks its own boundary each frame, exactly as a driver would from `on_frame`.
        let scope = pipeline.deferred_scope(animated.scope());
        let _subscription = vsync.on_frame(move |_| scope.mark_needs_paint());

        for _ in 0..3 {
            vsync.tick(Duration::from_millis(16));
            flush_paint(&mut pipeline);
        }

        assert_eq!(
            animated_paints.get(),
            4,
            "the animated boundary repainted on each of the three frames"
        );
        assert_eq!(
            static_paints.get(),
            1,
            "the static boundary was painted once and reused throughout"
        );
    }

    #[test]
    fn a_boundary_torn_down_while_painting_does_not_re_enter_the_flush() {
        // The root, while painting, drops a nested boundary's handle, the way a dynamic teardown would.
        // Dropping the handle unregisters the boundary, which re-enters the pipeline, so the flush must
        // release its borrow before painting or this panics.
        struct Teardown {
            nested: Rc<RefCell<Option<PaintBoundaryHandle>>>,
        }

        impl RenderObject for Teardown {
            fn mount(&mut self, _: &mut MountCtx) {}
            fn unmount(&mut self, _: &mut MountCtx) {}
        }

        impl RenderBox for Teardown {
            trivial_box_layout!();

            fn update_compositing_bits(&mut self) -> bool {
                false
            }

            fn paint(&mut self, _: &mut PaintCtx, _: Offset) {
                self.nested.borrow_mut().take();
            }
        }

        let slot: Rc<RefCell<Option<PaintBoundaryHandle>>> = Rc::new(RefCell::new(None));

        let (mut pipeline, root) = PaintPipeline::new(
            content(Teardown {
                nested: Rc::clone(&slot),
            }),
            layer(),
        );

        let nested_paints = Rc::new(Cell::new(0));
        let nested = pipeline.register_boundary(
            root.scope(),
            content(Counter {
                paints: Rc::clone(&nested_paints),
                color: Color::BLACK,
            }),
            layer(),
        );
        *slot.borrow_mut() = Some(nested);

        // The root, at depth 0, paints first and drops the nested boundary. Reaching the nested id after
        // that resolves to nothing, so it is skipped rather than painted from a stale node.
        flush_paint(&mut pipeline);

        assert!(
            slot.borrow().is_none(),
            "the root dropped the nested handle while painting"
        );
        assert_eq!(
            nested_paints.get(),
            0,
            "the torn-down boundary did not paint"
        );
    }
}

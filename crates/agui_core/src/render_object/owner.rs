use std::{cell::RefCell, rc::Rc};

use fnv::FnvHashSet;
use slotmap::{SlotMap, new_key_type};

use crate::{
    constraints::Constraints,
    paint::{Compositor, ContainerLayer, LayerHandle, PaintCtx, Scene},
    render_object::{
        RenderObject,
        box_layout::{AnyRenderBox, RenderBox},
    },
    size::Size,
};

new_key_type! {
    /// Identifies one repaint boundary within a [`RenderOwner`].
    pub struct BoundaryId;
}

/// Owns the repaint boundaries of a render forest and repaints the ones that have changed.
///
/// Each boundary is a subtree that paints into its own retained layer. A boundary can be repainted on
/// its own, leaving every other boundary's layer untouched, so a change confined to one boundary costs
/// only that boundary's paint. Mark a boundary through the [`PaintScope`] handed back when it is
/// registered.
pub struct RenderOwner {
    boundaries: SlotMap<BoundaryId, Boundary>,
    pending: Rc<RefCell<Pending>>,
    /// The layer the root view paints into and the creator composites to present.
    root_layer: LayerHandle<ContainerLayer>,
    /// The root view's boundary, once one is registered.
    root_boundary: Option<PaintBoundaryHandle>,
}

/// The drawable content of a boundary, shared between its render object and the owner.
pub type BoundaryContent = Rc<RefCell<Box<dyn AnyRenderBox>>>;

struct Boundary {
    content: BoundaryContent,
    layer: LayerHandle<ContainerLayer>,
}

/// The boundaries awaiting repaint, plus the hook that asks the driver to schedule a frame when the
/// first one becomes dirty on an otherwise clean owner.
#[derive(Default)]
struct Pending {
    dirty: FnvHashSet<BoundaryId>,
    notify: Option<Box<dyn Fn()>>,
}

impl RenderOwner {
    pub fn new() -> Self {
        Self {
            boundaries: SlotMap::with_key(),
            pending: Rc::new(RefCell::new(Pending::default())),
            root_layer: LayerHandle::new(ContainerLayer::new()),
            root_boundary: None,
        }
    }

    /// Sets the hook called when the first boundary becomes dirty on an otherwise clean owner, so the
    /// driver can schedule a frame. It does not fire for further marks until the next
    /// [`flush_paint`](RenderOwner::flush_paint) clears the dirty set. Replacing it drops the previous.
    pub fn on_needs_visual_update(&mut self, callback: impl Fn() + 'static) {
        self.pending.borrow_mut().notify = Some(Box::new(callback));
    }

    /// Adds a boundary that paints `content` into `layer`, returning a [`PaintBoundaryHandle`] that owns
    /// the boundary and hands out [`PaintScope`]s for marking it.
    ///
    /// # Panics
    ///
    /// Panics if called while a paint pass is in progress.
    pub fn register(
        &mut self,
        content: BoundaryContent,
        layer: LayerHandle<ContainerLayer>,
    ) -> PaintBoundaryHandle {
        let id = self.boundaries.insert(Boundary { content, layer });

        // A fresh boundary has never been painted, so it is dirty until its first frame.
        self.pending
            .try_borrow_mut()
            .expect("boundaries cannot be registered during paint")
            .dirty
            .insert(id);

        tracing::debug!(boundary = ?id, "registered repaint boundary");

        PaintBoundaryHandle {
            scope: PaintScope {
                id,
                pending: Rc::clone(&self.pending),
            },
        }
    }

    /// Removes a boundary, discarding its layer and clearing any pending mark. The handle is spent.
    ///
    /// # Panics
    ///
    /// Panics if called while a paint pass is in progress.
    pub fn unregister(&mut self, handle: PaintBoundaryHandle) {
        let PaintBoundaryHandle {
            scope: PaintScope { id, pending: _ },
        } = handle;

        self.boundaries.remove(id);

        self.pending
            .try_borrow_mut()
            .expect("boundaries cannot be unregistered during paint")
            .dirty
            .remove(&id);

        tracing::debug!(boundary = ?id, "unregistered repaint boundary");

        drop(handle);
    }

    /// Repaints every boundary that is unpainted or marked, leaving the rest as they are.
    ///
    /// # Panics
    ///
    /// Panics if called while another paint pass is already in progress.
    pub fn flush_paint(&mut self) {
        let pending = Rc::clone(&self.pending);
        let mut pending = pending
            .try_borrow_mut()
            .expect("cannot flush paint during paint");

        tracing::debug!(count = pending.dirty.len(), "flushing paint");

        for id in pending.dirty.drain() {
            self.repaint(id);
        }
    }

    fn repaint(&mut self, id: BoundaryId) {
        tracing::debug!(boundary = ?id, "repainting boundary");

        let boundary = &self.boundaries[id];
        let content = Rc::clone(&boundary.content);
        let layer = boundary.layer.clone();

        layer.borrow_mut().clear();
        PaintCtx::paint(&layer, |ctx| content.borrow_mut().paint(ctx));
    }

    /// Mounts `content` as the root view: registers it as the root boundary painting into the root
    /// layer, then mounts its subtree under that boundary's scope.
    pub fn mount_view(&mut self, content: Box<dyn AnyRenderBox>) {
        let content: BoundaryContent = Rc::new(RefCell::new(content));
        let handle = self.register(Rc::clone(&content), self.root_layer.clone());
        let scope = handle.scope();
        self.root_boundary = Some(handle);

        let mut ctx = MountCtx::new(self, scope);
        content.borrow_mut().mount(&mut ctx);
    }

    /// Unmounts the root view and unregisters its boundary.
    pub fn unmount_view(&mut self) {
        let Some(handle) = self.root_boundary.take() else {
            return;
        };

        let content = Rc::clone(&self.boundaries[handle.scope.id].content);
        {
            let mut ctx = MountCtx::new(self, handle.scope());
            content.borrow_mut().unmount(&mut ctx);
        }

        self.unregister(handle);
    }

    /// Lays the root view's subtree out against `constraints`, the target's current size.
    ///
    /// # Panics
    ///
    /// Panics if no view has been registered.
    pub fn layout(&mut self, constraints: Constraints) -> Size {
        let id = self
            .root_boundary
            .as_ref()
            .expect("a view must be registered before layout")
            .scope
            .id;
        self.boundaries[id].content.borrow_mut().layout(constraints)
    }

    /// Marks the root view for repaint on the next [`flush_paint`](RenderOwner::flush_paint).
    pub fn mark_needs_paint(&self) {
        if let Some(boundary) = &self.root_boundary {
            boundary.mark_needs_paint();
        }
    }

    /// Composites the root view's retained layers into a scene to present.
    pub fn composite(&self) -> Scene {
        Compositor::compose(&self.root_layer)
    }
}

impl Default for RenderOwner {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MountCtx<'a> {
    owner: &'a mut RenderOwner,
    paint_scope: PaintScope,
}

impl<'a> MountCtx<'a> {
    pub fn new(owner: &'a mut RenderOwner, paint_scope: PaintScope) -> Self {
        Self { owner, paint_scope }
    }

    /// Adds a boundary that paints `content` into `layer`, returning the [`PaintBoundaryHandle`] that
    /// owns it.
    pub fn register_boundary(
        &mut self,
        content: BoundaryContent,
        layer: LayerHandle<ContainerLayer>,
    ) -> PaintBoundaryHandle {
        self.owner.register(content, layer)
    }

    /// Removes a boundary that is leaving the tree.
    pub fn unregister_boundary(&mut self, handle: PaintBoundaryHandle) {
        self.owner.unregister(handle);
    }

    /// The [`PaintScope`] of the nearest enclosing boundary.
    pub fn paint_scope(&self) -> &PaintScope {
        &self.paint_scope
    }

    /// Mounts a subtree with `scope` as its enclosing boundary, restoring the previous scope afterward.
    /// A boundary calls this so its descendants repaint into it rather than into its own parent.
    pub fn with_paint_scope(&mut self, scope: PaintScope, f: impl FnOnce(&mut Self)) {
        let previous = std::mem::replace(&mut self.paint_scope, scope);
        f(self);
        self.paint_scope = previous;
    }
}

/// The boundary a render object repaints into. A node deep in a subtree holds the scope of its nearest
/// enclosing boundary and marks it when its painting goes stale; the boundary then repaints on the next
/// frame, leaving every other boundary untouched. Cloning shares the same target, so the scope can be
/// marked from anywhere, including a per-frame callback. A scope can only mark its boundary, never
/// remove it, so it is safe to hand down a subtree.
#[derive(Clone)]
pub struct PaintScope {
    id: BoundaryId,
    pending: Rc<RefCell<Pending>>,
}

impl PaintScope {
    /// Marks the boundary to be repainted on the next [`flush_paint`](RenderOwner::flush_paint). If
    /// this is the first mark on an otherwise clean owner, the owner's visual-update hook fires so the
    /// driver schedules a frame.
    pub fn mark_needs_paint(&self) {
        tracing::trace!(boundary = ?self.id, "marked boundary for repaint");

        let mut pending = self.pending.borrow_mut();
        let was_clean = pending.dirty.is_empty();
        pending.dirty.insert(self.id);

        if was_clean && let Some(notify) = &pending.notify {
            notify();
        }
    }
}

/// A registered boundary, returned to the render object that registered it. It owns the boundary's
/// place in the owner and is the only thing [`unregister`](RenderOwner::unregister) accepts; it hands
/// out mark-only [`PaintScope`]s for the subtree. Keeping removal here, off the scope, stops a
/// descendant that was handed a scope to mark with from unregistering the boundary it lives under.
pub struct PaintBoundaryHandle {
    scope: PaintScope,
}

impl PaintBoundaryHandle {
    /// A mark-only handle to this boundary, for descendants to repaint into it.
    pub fn scope(&self) -> PaintScope {
        self.scope.clone()
    }

    /// Marks this boundary to be repainted on the next [`flush_paint`](RenderOwner::flush_paint).
    pub fn mark_needs_paint(&self) {
        self.scope.mark_needs_paint();
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
        constraints::Constraints,
        hit_test::{HitTest, HitTestResult},
        offset::Offset,
        paint::{
            Compositor, PaintCommand, Scene,
            peniko::{Color, Fill},
        },
        rect::Rect,
        render_object::RenderObject,
        size::Size,
        text_baseline::TextBaseline,
        vsync::Vsync,
    };

    use super::*;

    /// A leaf that counts its paints and fills a unit square, so a test can tell whether it repainted.
    struct Counter {
        paints: Rc<Cell<usize>>,
        color: Color,
    }

    /// A boundary content that fills, then embeds the layers of nested boundaries — the stand-in for a
    /// render object that hosts child repaint boundaries.
    struct Embedder {
        paints: Rc<Cell<usize>>,
        color: Color,
        children: Vec<LayerHandle<ContainerLayer>>,
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
            fn measure(&self, _: Constraints) -> Size {
                Size::new(1.0, 1.0)
            }
            fn layout(&mut self, _: Constraints) -> Size {
                Size::new(1.0, 1.0)
            }
            fn measure_baseline(
                &self,
                _: Constraints,
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

        fn paint(&mut self, ctx: &mut PaintCtx) {
            self.paints.set(self.paints.get() + 1);

            let mut canvas = ctx.canvas();
            let brush = canvas.brush(self.color);
            canvas.fill(Fill::NonZero, brush, &Rect::from(Size::new(1.0, 1.0)));
        }
    }

    impl RenderObject for Embedder {
        fn mount(&mut self, _: &mut MountCtx) {}
        fn unmount(&mut self, _: &mut MountCtx) {}
    }

    impl RenderBox for Embedder {
        trivial_box_layout!();

        fn paint(&mut self, ctx: &mut PaintCtx) {
            self.paints.set(self.paints.get() + 1);

            {
                let mut canvas = ctx.canvas();
                let brush = canvas.brush(self.color);
                canvas.fill(Fill::NonZero, brush, &Rect::from(Size::new(1.0, 1.0)));
            }

            for child in &self.children {
                ctx.add_layer(child.clone().into());
            }
        }
    }

    fn content(render: impl AnyRenderBox + 'static) -> BoundaryContent {
        Rc::new(RefCell::new(Box::new(render) as Box<dyn AnyRenderBox>))
    }

    fn layer() -> LayerHandle<ContainerLayer> {
        LayerHandle::new(ContainerLayer::new())
    }

    fn fills(scene: &Scene) -> usize {
        scene
            .flatten()
            .commands()
            .iter()
            .filter(|command| matches!(command, PaintCommand::Fill { .. }))
            .count()
    }

    /// Marking one boundary repaints only it; the clean parent boundary is not repainted, yet the
    /// composed scene still contains the child because the parent embeds it by its retained layer.
    #[test]
    fn marking_a_boundary_repaints_only_it() {
        let root_paints = Rc::new(Cell::new(0));
        let child_paints = Rc::new(Cell::new(0));

        let mut owner = RenderOwner::new();

        let root_layer = layer();
        let child_layer = layer();
        owner.register(
            content(Embedder {
                paints: Rc::clone(&root_paints),
                color: Color::BLACK,
                children: vec![child_layer.clone()],
            }),
            root_layer.clone(),
        );
        let child = owner.register(
            content(Counter {
                paints: Rc::clone(&child_paints),
                color: Color::rgb8(255, 0, 0),
            }),
            child_layer,
        );

        owner.flush_paint();
        let first = Compositor::compose(&root_layer);
        assert_eq!(root_paints.get(), 1);
        assert_eq!(child_paints.get(), 1);
        assert_eq!(
            fills(&first),
            2,
            "the root and child both contributed a fill"
        );

        // The kind of out-of-band mark a per-frame animation callback would make.
        child.mark_needs_paint();

        owner.flush_paint();
        let second = Compositor::compose(&root_layer);
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

    /// A second frame with nothing marked repaints nothing.
    #[test]
    fn a_clean_frame_repaints_nothing() {
        let paints = Rc::new(Cell::new(0));

        let mut owner = RenderOwner::new();
        owner.register(
            content(Counter {
                paints: Rc::clone(&paints),
                color: Color::BLACK,
            }),
            layer(),
        );

        owner.flush_paint();
        owner.flush_paint();

        assert_eq!(
            paints.get(),
            1,
            "an unmarked boundary paints once and is reused"
        );
    }

    /// The visual-update hook fires on the clean-to-dirty edge so the driver schedules a frame, and not
    /// again until a flush clears the dirty set.
    #[test]
    fn marking_schedules_a_frame_on_the_clean_to_dirty_edge() {
        let mut owner = RenderOwner::new();
        let scope = owner.register(
            content(Counter {
                paints: Rc::new(Cell::new(0)),
                color: Color::BLACK,
            }),
            layer(),
        );

        let frames = Rc::new(Cell::new(0));
        let scheduled = Rc::clone(&frames);
        owner.on_needs_visual_update(move || scheduled.set(scheduled.get() + 1));

        // Registration left the owner dirty; the first frame clears it without involving the hook.
        owner.flush_paint();
        assert_eq!(frames.get(), 0, "registration alone schedules no frame");

        scope.mark_needs_paint();
        scope.mark_needs_paint();
        assert_eq!(
            frames.get(),
            1,
            "only the clean-to-dirty edge schedules a frame"
        );

        owner.flush_paint();
        scope.mark_needs_paint();
        assert_eq!(
            frames.get(),
            2,
            "a mark after the flush schedules another frame"
        );
    }

    /// The whole loop: an animation marks its boundary from a per-frame callback, and each frame
    /// repaints only that boundary while a static sibling is painted once and then reused.
    #[test]
    fn an_animation_repaints_only_its_own_boundary() {
        let vsync = Vsync::new();

        let static_paints = Rc::new(Cell::new(0));
        let animated_paints = Rc::new(Cell::new(0));

        let mut owner = RenderOwner::new();

        let static_layer = layer();
        let animated_layer = layer();
        owner.register(
            content(Embedder {
                paints: Rc::new(Cell::new(0)),
                color: Color::WHITE,
                children: vec![static_layer.clone(), animated_layer.clone()],
            }),
            layer(),
        );
        owner.register(
            content(Counter {
                paints: Rc::clone(&static_paints),
                color: Color::BLACK,
            }),
            static_layer,
        );
        let animated = owner.register(
            content(Counter {
                paints: Rc::clone(&animated_paints),
                color: Color::rgb8(255, 0, 0),
            }),
            animated_layer,
        );

        owner.flush_paint();
        assert_eq!(static_paints.get(), 1);
        assert_eq!(animated_paints.get(), 1);

        // The animation marks its own boundary each frame, exactly as a driver would from `on_frame`.
        let scope = animated.scope();
        let _subscription = vsync.on_frame(move |_| scope.mark_needs_paint());

        for _ in 0..3 {
            vsync.tick(Duration::from_millis(16));
            owner.flush_paint();
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
}

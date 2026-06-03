use std::{cell::RefCell, rc::Rc};

use fnv::FnvHashSet;
use slotmap::{SlotMap, new_key_type};

use crate::{
    paint::{Compositor, ContainerLayer, LayerHandle, PaintCtx, Scene},
    render_object::box_layout::{AnyRenderBox, RenderBox},
};

new_key_type! {
    /// Identifies one repaint boundary within a [`RepaintOwner`].
    pub struct BoundaryId;
}

/// Owns the repaint boundaries of a render forest and repaints the ones that have changed.
///
/// Each boundary is a subtree that paints into its own retained layer. A boundary can be repainted on
/// its own, leaving every other boundary's layer untouched, so a change confined to one boundary costs
/// only that boundary's paint. Mark a boundary through the [`PaintScope`] handed back when it is
/// registered.
pub struct RepaintOwner {
    boundaries: SlotMap<BoundaryId, Boundary>,
    dirty: Rc<RefCell<FnvHashSet<BoundaryId>>>,
}

/// The drawable content of a boundary, shared between its render object and the owner.
pub type BoundaryContent = Rc<RefCell<Box<dyn AnyRenderBox>>>;

struct Boundary {
    content: BoundaryContent,
    layer: LayerHandle<ContainerLayer>,
}

impl RepaintOwner {
    pub fn new() -> Self {
        Self {
            boundaries: SlotMap::with_key(),
            dirty: Rc::new(RefCell::new(FnvHashSet::default())),
        }
    }

    /// Adds a boundary that paints `content` into `layer`, returning a handle for marking it for
    /// repaint.
    ///
    /// # Panics
    ///
    /// Panics if called while a paint pass is in progress.
    pub fn register(
        &mut self,
        content: BoundaryContent,
        layer: LayerHandle<ContainerLayer>,
    ) -> PaintScope {
        let id = self.boundaries.insert(Boundary { content, layer });

        // A fresh boundary has never been painted, so it is dirty until its first frame.
        self.dirty
            .try_borrow_mut()
            .expect("boundaries cannot be registered during paint")
            .insert(id);

        tracing::debug!(boundary = ?id, "registered repaint boundary");

        PaintScope {
            id,
            dirty: Rc::clone(&self.dirty),
        }
    }

    /// Removes a boundary, discarding its layer and clearing any pending mark. The handle is spent.
    ///
    /// # Panics
    ///
    /// Panics if called while a paint pass is in progress.
    pub fn unregister(&mut self, handle: PaintScope) {
        let PaintScope { id, dirty: _ } = handle;

        self.boundaries.remove(id);

        self.dirty
            .try_borrow_mut()
            .expect("boundaries cannot be unregistered during paint")
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
        let dirty = Rc::clone(&self.dirty);
        let mut dirty_set = dirty
            .try_borrow_mut()
            .expect("cannot flush paint during paint");

        tracing::debug!(count = dirty_set.len(), "flushing paint");

        for id in dirty_set.drain() {
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

    /// Composes the boundary `handle` refers to — and everything it embeds — into a scene for one
    /// render target. Call after [`flush_paint`](RepaintOwner::flush_paint); call once per target.
    pub fn compose(&self, handle: &PaintScope) -> Scene {
        tracing::trace!(boundary = ?handle.id, "composing render target");

        Compositor::compose(&self.boundaries[handle.id].layer)
    }
}

impl Default for RepaintOwner {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MountCtx<'a> {
    owner: &'a mut RepaintOwner,
    paint_scope: Option<PaintScope>,
}

impl<'a> MountCtx<'a> {
    pub fn new(owner: &'a mut RepaintOwner) -> Self {
        Self {
            owner,
            paint_scope: None,
        }
    }

    /// Adds a boundary that paints `content` into `layer`, returning the [`PaintScope`] the boundary
    /// repaints into.
    pub fn register_boundary(
        &mut self,
        content: BoundaryContent,
        layer: LayerHandle<ContainerLayer>,
    ) -> PaintScope {
        self.owner.register(content, layer)
    }

    /// Removes a boundary that is leaving the tree.
    pub fn unregister_boundary(&mut self, scope: PaintScope) {
        self.owner.unregister(scope);
    }

    /// The [`PaintScope`] of the nearest enclosing boundary, if the node being mounted is under one.
    pub fn paint_scope(&self) -> Option<&PaintScope> {
        self.paint_scope.as_ref()
    }

    /// Mounts a subtree with `scope` as its enclosing boundary, restoring the previous scope afterward.
    /// A boundary calls this so its descendants repaint into it rather than into its own parent.
    pub fn with_paint_scope(&mut self, scope: PaintScope, f: impl FnOnce(&mut Self)) {
        let previous = self.paint_scope.replace(scope);
        f(self);
        self.paint_scope = previous;
    }
}

/// The boundary a render object repaints into. A node deep in a subtree holds the scope of its nearest
/// enclosing boundary and marks it when its painting goes stale; the boundary then repaints on the next
/// frame, leaving every other boundary untouched. Cloning shares the same target, so the scope can be
/// marked from anywhere, including a per-frame callback.
#[derive(Clone)]
pub struct PaintScope {
    id: BoundaryId,
    dirty: Rc<RefCell<FnvHashSet<BoundaryId>>>,
}

impl PaintScope {
    /// Marks the boundary to be repainted on the next [`flush_paint`](RepaintOwner::flush_paint).
    pub fn mark_needs_paint(&self) {
        tracing::trace!(boundary = ?self.id, "marked boundary for repaint");

        self.dirty.borrow_mut().insert(self.id);
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
            PaintCommand,
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

        let mut owner = RepaintOwner::new();

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

        let mut owner = RepaintOwner::new();
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

    /// The whole loop: an animation marks its boundary from a per-frame callback, and each frame
    /// repaints only that boundary while a static sibling is painted once and then reused.
    #[test]
    fn an_animation_repaints_only_its_own_boundary() {
        let vsync = Vsync::new();

        let static_paints = Rc::new(Cell::new(0));
        let animated_paints = Rc::new(Cell::new(0));

        let mut owner = RepaintOwner::new();

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
        let scope = animated.clone();
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

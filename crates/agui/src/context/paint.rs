use std::cell::Cell;
use std::rc::Rc;

use peniko::kurbo::Affine;

use crate::{
    geometry::Offset,
    paint::{
        Canvas,
        command::PaintCommand,
        compositing::{
            ContainerLayer, LayerHandle, OffsetLayer, PictureLayer, PositionedLayer, TransformLayer,
        },
        scene::{Scene, SceneCapacity},
    },
    pipeline::render_pipeline::{
        CompositingBitsHook, DeferredPaintScope, PaintBoundaryHandle, PaintScope, PaintState,
        RepaintHook,
    },
    render_object::{box_layout::RenderBox, node::MountedChild},
};

/// A paint's remaining buffer capacity and the lengths recorded so far, shared by every picture
/// the paint seals.
struct PaintBudget {
    remaining: SceneCapacity,
    recorded: SceneCapacity,
}

/// The surface a render object paints onto.
///
/// It accepts flat drawing through [`canvas`](PaintCtx::canvas) and retained layers through
/// [`push_layer`](PaintCtx::push_layer) or [`add_layer`](PaintCtx::add_layer), preserving the order
/// they are issued in. A render object positions itself by the `offset` passed to its paint and draws
/// at that offset, so a translation costs nothing here. For a genuine transform like a rotation, use
/// [`with_transform`](PaintCtx::with_transform), which brackets the enclosed drawing and any layer
/// contributed within it.
pub struct PaintCtx<'a> {
    /// Where sealed pictures and contributed layers are appended.
    container: &'a mut dyn ContainerLayer,
    /// The picture currently accumulating flat drawing.
    picture: Scene,
    /// The capacity budget shared with any nested context of the same paint.
    budget: &'a mut PaintBudget,
    /// The boundary registry, to register a repaint boundary discovered during paint and to mark one.
    paint: &'a Rc<PaintState>,
    /// The repaint boundary this paint is drawing into. A node stores it to repaint that boundary when its
    /// painting goes stale, and forwards it to the children it paints.
    scope: PaintScope,
}

impl PaintCtx<'_> {
    /// Paints `build` into `root` as the content of repaint boundary `scope`, against `paint`.
    pub(crate) fn paint(
        root: &LayerHandle<impl ContainerLayer>,
        paint: &Rc<PaintState>,
        scope: PaintScope,
        build: impl FnOnce(&mut PaintCtx),
    ) {
        Self::paint_with_capacity(root, SceneCapacity::default(), paint, scope, build);
    }

    /// Paints `build` into `root` as the content of repaint boundary `scope`, sizing the recording buffers
    /// from `capacity`, and returns the lengths recorded.
    ///
    /// A caller that repaints the same content passes the lengths returned by the previous paint,
    /// so a recording of similar size fills pre-sized buffers instead of growing them.
    pub(crate) fn paint_with_capacity(
        root: &LayerHandle<impl ContainerLayer>,
        capacity: SceneCapacity,
        paint: &Rc<PaintState>,
        scope: PaintScope,
        build: impl FnOnce(&mut PaintCtx),
    ) -> SceneCapacity {
        let mut root = root.borrow_mut();

        let mut budget = PaintBudget {
            remaining: capacity,
            recorded: SceneCapacity::default(),
        };

        let mut ctx = PaintCtx {
            container: &mut *root,
            picture: Scene::new(),
            budget: &mut budget,
            paint,
            scope,
        };

        build(&mut ctx);

        ctx.flush();

        budget.recorded
    }

    /// A [`Canvas`] for flat drawing, ordered before any layer contributed after this call.
    pub fn canvas(&mut self) -> Canvas<'_> {
        self.picture.reserve(self.budget.remaining);

        Canvas::over(&mut self.picture)
    }

    /// Records the enclosed painting under `transform`, concatenated onto the transform already in
    /// effect.
    ///
    /// `needs_compositing` determines whether the enclosed subtree contributes a retained layer: when
    /// it does, the transform is applied at the layer level so a composited child is transformed
    /// correctly. Otherwise, it is a flat transform over the drawing.
    pub fn with_transform(
        &mut self,
        needs_compositing: bool,
        transform: Affine,
        f: impl FnOnce(&mut PaintCtx),
    ) {
        if needs_compositing {
            self.flush();

            let layer = LayerHandle::new(TransformLayer::new(transform));
            {
                let mut guard = layer.borrow_mut();

                let mut ctx = PaintCtx {
                    container: &mut *guard,
                    picture: Scene::new(),
                    budget: &mut *self.budget,
                    paint: self.paint,
                    scope: self.scope,
                };

                f(&mut ctx);

                ctx.flush();
            }

            self.container.append(layer.into());

            return;
        }

        self.picture.push(PaintCommand::PushTransform(transform));
        {
            f(self);
        }
        self.picture.push(PaintCommand::PopTransform);
    }

    /// Contributes a retained layer at `offset` and paints `paint_into` as its content. Use it for a
    /// subtree worth keeping across frames, so it can be reused without repainting. The content is
    /// painted in the layer's own coordinates, so `paint_into` should paint at [`Offset::ZERO`].
    pub fn push_layer<L: ContainerLayer + PositionedLayer + 'static>(
        &mut self,
        layer: LayerHandle<L>,
        offset: Offset,
        paint_into: impl FnOnce(&mut PaintCtx),
    ) {
        self.flush();

        {
            let mut guard = layer.borrow_mut();
            guard.set_offset(offset);

            let mut ctx = PaintCtx {
                container: &mut *guard,
                picture: Scene::new(),
                budget: &mut *self.budget,
                paint: self.paint,
                scope: self.scope,
            };
            paint_into(&mut ctx);
            ctx.flush();
        }

        self.container.append(layer.into());
    }

    /// Contributes an already-built retained layer at `offset`, painting nothing into it. Use it to
    /// reuse a layer whose content is unchanged.
    pub fn add_layer<L: PositionedLayer + 'static>(
        &mut self,
        layer: LayerHandle<L>,
        offset: Offset,
    ) {
        self.flush();

        layer.borrow_mut().set_offset(offset);
        self.container.append(layer.into());
    }

    /// The repaint boundary this paint is drawing into. A node forwards it to the children it paints and
    /// stores it to repaint that boundary when its own painting goes stale.
    pub fn scope(&self) -> PaintScope {
        self.scope
    }

    /// Registers the child reached by `content` as a repaint boundary nested under the boundary in force,
    /// painting into `layer`, and returns the handle that owns and marks it. A node that becomes a repaint
    /// boundary registers itself this way during paint, fills `layer` in the same pass with
    /// [`push_boundary_layer`](Self::push_boundary_layer), and on later frames embeds it with
    /// [`add_layer`](Self::add_layer); the driver repaints its content into `layer` on its own pass.
    pub fn register_paint_boundary(
        &self,
        content: MountedChild<dyn RenderBox>,
        layer: LayerHandle<OffsetLayer>,
    ) -> PaintBoundaryHandle {
        let paint = Rc::downgrade(self.paint);
        let capacity = Cell::new(SceneCapacity::default());

        let repaint: RepaintHook = Box::new(move |scope| {
            let Some(paint) = paint.upgrade() else {
                return;
            };

            layer.borrow_mut().clear();

            let recorded =
                PaintCtx::paint_with_capacity(&layer, capacity.get(), &paint, scope, |ctx| {
                    content.borrow_mut().paint(ctx, Offset::ZERO);
                });

            capacity.set(recorded);
        });

        let update_bits: CompositingBitsHook = Box::new(move || {
            content.borrow_mut().update_compositing_bits();
        });

        self.paint.register(self.scope, repaint, update_bits)
    }

    /// Paints `content` into `layer` as repaint boundary `scope`, then embeds the layer here at `offset`. A
    /// node that just registered itself as a repaint boundary uses this to fill its layer in the same pass, so
    /// the layer is not blank until the boundary's first isolated repaint.
    pub fn push_boundary_layer(
        &mut self,
        scope: PaintScope,
        layer: LayerHandle<OffsetLayer>,
        offset: Offset,
        content: impl FnOnce(&mut PaintCtx),
    ) {
        PaintCtx::paint(&layer, self.paint, scope, content);
        self.add_layer(layer, offset);
    }

    /// A deferred handle to the boundary in force, for marking it from a per-frame animation callback that
    /// holds no context.
    pub fn deferred_paint_scope(&self) -> DeferredPaintScope {
        self.paint.deferred_scope(self.scope)
    }

    /// Marks `scope`'s boundary to be repainted on the next frame.
    pub fn mark_needs_paint(&self, scope: PaintScope) {
        self.paint.mark_needs_paint(scope);
    }

    /// Marks `scope`'s compositing bits for recomputation before its next repaint, and the boundary for
    /// repaint.
    pub fn mark_needs_compositing_bits_update(&self, scope: PaintScope) {
        self.paint.mark_needs_compositing_bits_update(scope);
    }

    /// Schedules a recomposite of the subtree on the next frame, without repainting any boundary.
    pub fn mark_needs_composite(&self) {
        self.paint.mark_needs_composite();
    }

    /// Appends the flat drawing accumulated so far as a [`PictureLayer`], then starts a fresh picture.
    fn flush(&mut self) {
        if !self.picture.has_drawing() {
            return;
        }

        let picture = std::mem::take(&mut self.picture);

        let lengths = picture.lengths();
        self.budget.recorded.add(lengths);
        self.budget.remaining = self.budget.remaining.saturating_sub(lengths);

        self.container
            .append(LayerHandle::new(PictureLayer::new(picture)).into());
    }
}

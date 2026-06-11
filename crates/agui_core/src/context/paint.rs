use peniko::kurbo::Affine;

use crate::{
    geometry::Offset,
    paint::{
        Canvas,
        command::PaintCommand,
        compositing::{ContainerLayer, LayerHandle, PictureLayer, PositionedLayer, TransformLayer},
        scene::{Scene, SceneCapacity},
    },
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
}

impl PaintCtx<'_> {
    /// Paints `build` into `root`.
    pub fn paint(root: &LayerHandle<impl ContainerLayer>, build: impl FnOnce(&mut PaintCtx)) {
        Self::paint_with_capacity(root, SceneCapacity::default(), build);
    }

    /// Paints `build` into `root`, sizing the recording buffers from `capacity`, and returns the
    /// lengths actually recorded.
    ///
    /// A caller that repaints the same content passes the lengths returned by the previous paint,
    /// so a recording of similar size fills pre-sized buffers instead of growing them.
    pub fn paint_with_capacity(
        root: &LayerHandle<impl ContainerLayer>,
        capacity: SceneCapacity,
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

#[cfg(test)]
mod tests {
    use peniko::{Color, Fill, kurbo::Affine};

    use crate::{
        geometry::{Rect, Size},
        paint::{
            Canvas,
            command::PaintCommand,
            compositing::{Compositor, Layer, OffsetLayer},
        },
    };

    use super::*;

    fn root() -> LayerHandle<OffsetLayer> {
        LayerHandle::new(OffsetLayer::new())
    }

    fn fill(ctx: &mut PaintCtx) {
        let mut canvas = ctx.canvas();
        let brush = canvas.brush(Color::BLACK);
        canvas.fill(Fill::NonZero, brush, &Rect::from(Size::new(1.0, 1.0)));
    }

    /// A pre-built retained layer holding a single fill, for [`add_layer`](PaintContext::add_layer).
    fn fill_layer() -> LayerHandle<OffsetLayer> {
        let picture = Canvas::record(|canvas| {
            let brush = canvas.brush(Color::BLACK);
            canvas.fill(Fill::NonZero, brush, &Rect::from(Size::new(1.0, 1.0)));
        });

        let mut layer = OffsetLayer::new();
        layer.append(LayerHandle::new(PictureLayer::new(picture)).into());
        LayerHandle::new(layer)
    }

    /// The transform in effect at each fill of the composed, flattened scene, in order.
    fn fill_transforms(layer: &LayerHandle<impl Layer>) -> Vec<Affine> {
        let scene = Compositor::compose(layer).flatten();

        let mut current = Affine::IDENTITY;
        let mut stack = Vec::new();
        let mut transforms = Vec::new();
        for command in scene.commands() {
            match command {
                PaintCommand::PushTransform(t) => {
                    stack.push(current);
                    current *= *t;
                }
                PaintCommand::PopTransform => current = stack.pop().expect("balanced"),
                PaintCommand::Fill { .. } => transforms.push(current),
                _ => {}
            }
        }
        transforms
    }

    #[test]
    fn flat_drawing_seals_into_one_picture() {
        let root = root();
        PaintCtx::paint(&root, fill);

        assert_eq!(fill_transforms(&root), vec![Affine::IDENTITY]);
    }

    #[test]
    fn a_flat_transform_places_drawing_under_it() {
        let root = root();
        PaintCtx::paint(&root, |ctx| {
            ctx.with_transform(false, Affine::translate((5.0, 7.0)), fill);
        });

        assert_eq!(fill_transforms(&root), vec![Affine::translate((5.0, 7.0))]);
    }

    #[test]
    fn a_pushed_layer_carries_its_content() {
        let root = root();
        PaintCtx::paint(&root, |ctx| {
            ctx.push_layer(LayerHandle::new(OffsetLayer::new()), Offset::ZERO, fill);
        });

        assert_eq!(fill_transforms(&root), vec![Affine::IDENTITY]);
    }

    /// A layer added at an offset is positioned there, even though it escapes the current picture into
    /// the container.
    #[test]
    fn a_layer_added_at_an_offset_is_positioned_there() {
        let root = root();
        PaintCtx::paint(&root, |ctx| {
            ctx.add_layer(fill_layer(), Offset::new(3.0, 0.0));
        });

        assert_eq!(fill_transforms(&root), vec![Affine::translate((3.0, 0.0))]);
    }

    /// The reported capacity covers every picture of the paint, including one sealed inside a pushed
    /// layer, and a repaint primed with it records the same content.
    #[test]
    fn a_repaint_primed_with_the_recorded_capacity_paints_identically() {
        let paint = |ctx: &mut PaintCtx| {
            fill(ctx);
            ctx.push_layer(LayerHandle::new(OffsetLayer::new()), Offset::ZERO, fill);
            ctx.with_transform(false, Affine::translate((5.0, 0.0)), fill);
        };

        let first_root = root();
        let recorded = PaintCtx::paint_with_capacity(&first_root, SceneCapacity::default(), paint);
        assert_ne!(recorded, SceneCapacity::default());

        let second_root = root();
        let second = PaintCtx::paint_with_capacity(&second_root, recorded, paint);

        assert_eq!(second, recorded);
        assert_eq!(fill_transforms(&second_root), fill_transforms(&first_root));
    }

    /// Drawing, then a layer, then drawing, inside one transform bracket: both pictures stay under the
    /// transform and the layer lands between them.
    #[test]
    fn drawing_resumes_under_the_same_bracket_after_a_layer() {
        let root = root();
        PaintCtx::paint(&root, |ctx| {
            ctx.with_transform(true, Affine::translate((2.0, 0.0)), |ctx| {
                fill(ctx);
                ctx.add_layer(fill_layer(), Offset::ZERO);
                fill(ctx);
            });
        });

        assert_eq!(
            fill_transforms(&root),
            vec![
                Affine::translate((2.0, 0.0)),
                Affine::translate((2.0, 0.0)),
                Affine::translate((2.0, 0.0)),
            ]
        );
    }
}

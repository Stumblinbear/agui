use peniko::kurbo::Affine;

use crate::{
    offset::Offset,
    paint::{
        canvas::Canvas,
        compositing::{Container, LayerHandle, PictureLayer, TransformLayer},
        scene::{PaintCommand, Scene},
    },
};

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
    container: &'a mut dyn Container,
    /// The picture currently accumulating flat drawing.
    picture: Scene,
}

impl PaintCtx<'_> {
    /// Paints `build` into `root`.
    pub fn paint(root: &LayerHandle<impl Container>, build: impl FnOnce(&mut PaintCtx)) {
        let mut root = root.borrow_mut();

        let mut ctx = PaintCtx {
            container: &mut *root,
            picture: Scene::new(),
        };
        build(&mut ctx);
        ctx.flush();
    }

    /// A [`Canvas`] for flat drawing, ordered before any layer contributed after this call.
    pub fn canvas(&mut self) -> Canvas<'_> {
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
    pub fn push_layer<L: Container + 'static>(
        &mut self,
        layer: LayerHandle<L>,
        offset: Offset,
        paint_into: impl FnOnce(&mut PaintCtx),
    ) {
        self.flush();

        {
            let mut guard = layer.borrow_mut();
            let mut ctx = PaintCtx {
                container: &mut *guard,
                picture: Scene::new(),
            };
            paint_into(&mut ctx);
            ctx.flush();
        }

        self.place(layer.into(), offset);
    }

    /// Contributes an already-built retained layer at `offset`, painting nothing into it. Use it to
    /// reuse a layer whose content is unchanged.
    pub fn add_layer(&mut self, layer: LayerHandle, offset: Offset) {
        self.flush();
        self.place(layer, offset);
    }

    /// Appends `layer`, positioned at `offset`.
    fn place(&mut self, layer: LayerHandle, offset: Offset) {
        if offset == Offset::ZERO {
            self.container.append(layer);
            return;
        }

        let positioned = LayerHandle::new(TransformLayer::new(Affine::translate(offset)));
        positioned.borrow_mut().append(layer);
        self.container.append(positioned.into());
    }

    /// Appends the flat drawing accumulated so far as a [`PictureLayer`], then starts a fresh picture.
    fn flush(&mut self) {
        let has_drawing = self.picture.commands().iter().any(|command| {
            !matches!(
                command,
                PaintCommand::PushTransform(_) | PaintCommand::PopTransform
            )
        });

        if !has_drawing {
            return;
        }

        let picture = std::mem::take(&mut self.picture);

        self.container
            .append(LayerHandle::new(PictureLayer::new(picture)).into());
    }
}

#[cfg(test)]
mod tests {
    use peniko::{Color, Fill, kurbo::Affine};

    use crate::{
        paint::{Compositor, ContainerLayer, PaintCommand, compositing::Layer},
        rect::Rect,
        size::Size,
    };

    use super::*;

    fn root() -> LayerHandle<ContainerLayer> {
        LayerHandle::new(ContainerLayer::new())
    }

    fn fill(ctx: &mut PaintCtx) {
        let mut canvas = ctx.canvas();
        let brush = canvas.brush(Color::BLACK);
        canvas.fill(Fill::NonZero, brush, &Rect::from(Size::new(1.0, 1.0)));
    }

    /// A pre-built retained layer holding a single fill, for [`add_layer`](PaintContext::add_layer).
    fn fill_layer() -> LayerHandle {
        let picture = Canvas::record(|canvas| {
            let brush = canvas.brush(Color::BLACK);
            canvas.fill(Fill::NonZero, brush, &Rect::from(Size::new(1.0, 1.0)));
        });

        LayerHandle::new(PictureLayer::new(picture)).into()
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
            ctx.push_layer(LayerHandle::new(ContainerLayer::new()), Offset::ZERO, fill);
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

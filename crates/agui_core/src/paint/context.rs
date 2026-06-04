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
/// It accepts flat drawing through [`canvas`](PaintContext::canvas) and retained layers through
/// [`push_layer`](PaintContext::push_layer) or [`add_layer`](PaintContext::add_layer), preserving the
/// order they are issued in. Drawing is in local coordinates;
/// [`with_offset`](PaintContext::with_offset) and [`with_transform`](PaintContext::with_transform)
/// place the enclosed drawing — and any layer contributed within — under a transform.
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

    /// Translates the coordinate system by `offset` for the enclosed painting. See
    /// [`with_transform`](PaintCtx::with_transform) for `needs_compositing`.
    pub fn with_offset(
        &mut self,
        needs_compositing: bool,
        offset: Offset,
        f: impl FnOnce(&mut PaintCtx),
    ) {
        self.with_transform(needs_compositing, Affine::translate(offset), f);
    }

    /// Contributes a retained layer and paints `paint_into` as its content. Use it for a subtree worth
    /// keeping across frames, so it can be reused without repainting.
    pub fn push_layer<L: Container + 'static>(
        &mut self,
        layer: LayerHandle<L>,
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

        self.container.append(layer.into());
    }

    /// Contributes an already-built retained layer, painting nothing into it. Use it to reuse a layer
    /// whose content is unchanged.
    pub fn add_layer(&mut self, layer: LayerHandle) {
        self.flush();
        self.container.append(layer);
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
    fn an_offset_places_flat_drawing_under_it() {
        let root = root();
        PaintCtx::paint(&root, |ctx| {
            ctx.with_offset(false, Offset::new(5.0_f32, 7.0_f32), fill);
        });

        assert_eq!(fill_transforms(&root), vec![Affine::translate((5.0, 7.0))]);
    }

    #[test]
    fn a_pushed_layer_carries_its_content() {
        let root = root();
        PaintCtx::paint(&root, |ctx| {
            ctx.push_layer(LayerHandle::new(ContainerLayer::new()), fill);
        });

        assert_eq!(fill_transforms(&root), vec![Affine::IDENTITY]);
    }

    /// A layer contributed inside a transform bracket is placed under that transform, even though it
    /// escapes the current picture into the container.
    #[test]
    fn a_layer_inside_a_bracket_is_placed_under_it() {
        let root = root();
        PaintCtx::paint(&root, |ctx| {
            ctx.with_offset(true, Offset::new(3.0_f32, 0.0_f32), |ctx| {
                ctx.add_layer(fill_layer());
            });
        });

        assert_eq!(fill_transforms(&root), vec![Affine::translate((3.0, 0.0))]);
    }

    /// Drawing, then a layer, then drawing, inside one bracket: both pictures stay under the
    /// transform and the layer lands between them.
    #[test]
    fn drawing_resumes_under_the_same_bracket_after_a_layer() {
        let root = root();
        PaintCtx::paint(&root, |ctx| {
            ctx.with_offset(true, Offset::new(2.0_f32, 0.0_f32), |ctx| {
                fill(ctx);
                ctx.add_layer(fill_layer());
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

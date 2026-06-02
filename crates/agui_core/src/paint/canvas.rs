use std::marker::PhantomData;

use peniko::{
    BlendMode, Brush, Fill,
    kurbo::{Affine, Shape, Stroke},
};

use crate::{
    offset::Offset,
    paint::scene::{PaintCommand, PaintShape, Scene},
};

/// A handle to a brush registered on a [`Canvas`]. Valid only within the paint pass that produced it.
#[derive(Clone, Copy)]
pub struct BrushId<'b> {
    index: u32,

    _phantom: PhantomData<&'b ()>,
}

/// A handle to a stroke style registered on a [`Canvas`]. Valid only within the paint pass that produced it.
#[derive(Clone, Copy)]
pub struct StrokeId<'b> {
    index: u32,

    _phantom: PhantomData<&'b ()>,
}

/// An interface for recording drawing operations during a paint pass.
///
/// A render object registers its brushes and stroke styles to get handles, then issues fills,
/// strokes, and clips, which accumulate into a [`Scene`] instead of drawing immediately. Obtain a
/// canvas through [`Canvas::record`].
pub struct Canvas<'b> {
    scene: &'b mut Scene,
    transform: Affine,
}

impl Canvas<'_> {
    /// Records a paint pass and returns the resulting [`Scene`].
    pub fn record(build: impl for<'b> FnOnce(&mut Canvas<'b>)) -> Scene {
        let mut scene = Scene::new();

        let mut canvas = Canvas {
            scene: &mut scene,
            transform: Affine::IDENTITY,
        };

        build(&mut canvas);

        scene
    }

    /// Records a paint pass into `scene`, clearing it first.
    pub fn record_into(scene: &mut Scene, build: impl for<'b> FnOnce(&mut Canvas<'b>)) {
        scene.reset();

        let mut canvas = Canvas {
            scene,
            transform: Affine::IDENTITY,
        };

        build(&mut canvas);
    }
}

impl<'b> Canvas<'b> {
    /// The current transform, from local coordinates to the scene's root.
    pub fn transform(&self) -> Affine {
        self.transform
    }

    /// Registers `brush` and returns a handle to it for use in [`Canvas::fill`] or [`Canvas::stroke`].
    pub fn brush(&mut self, brush: impl Into<Brush>) -> BrushId<'b> {
        BrushId {
            index: self.scene.intern_brush(brush.into()),
            _phantom: PhantomData,
        }
    }

    /// Registers `stroke` and returns a handle to it for use in [`Canvas::stroke`].
    pub fn stroke_style(&mut self, stroke: Stroke) -> StrokeId<'b> {
        StrokeId {
            index: self.scene.intern_stroke(stroke),
            _phantom: PhantomData,
        }
    }

    /// Concatenates `transform` onto the current transform for the enclosed drawing.
    pub fn with_transform(&mut self, transform: Affine, f: impl FnOnce(&mut Self)) {
        let prev = self.transform;

        self.transform *= transform;

        f(self);

        self.transform = prev;
    }

    /// Translates the coordinate system by `offset` for the enclosed drawing.
    pub fn with_offset(&mut self, offset: Offset, f: impl FnOnce(&mut Self)) {
        self.with_transform(Affine::translate(offset), f);
    }

    /// Renders the enclosed drawing into a separate layer, then composites it into the scene clipped
    /// to `clip`, blended with `blend`, at opacity `alpha`.
    pub fn with_layer(
        &mut self,
        blend: impl Into<BlendMode>,
        alpha: f32,
        clip: &impl Shape,
        f: impl FnOnce(&mut Self),
    ) {
        self.scene.push(PaintCommand::PushLayer {
            blend: blend.into(),
            alpha,
            transform: self.transform,
            clip: PaintShape::from_shape(clip),
        });

        f(self);

        self.scene.push(PaintCommand::PopLayer);
    }

    /// Clips the enclosed drawing to `clip`.
    pub fn with_clip(&mut self, clip: &impl Shape, f: impl FnOnce(&mut Self)) {
        self.with_layer(BlendMode::default(), 1.0, clip, f);
    }

    /// Fills `shape` with `brush`.
    pub fn fill(&mut self, style: Fill, brush: BrushId<'b>, shape: &impl Shape) {
        self.scene.push(PaintCommand::Fill {
            style,
            transform: self.transform,
            brush: brush.index,
            brush_transform: None,
            shape: PaintShape::from_shape(shape),
        });
    }

    /// Strokes the outline of `shape` with `brush`, using the registered `style`.
    pub fn stroke(&mut self, style: StrokeId<'b>, brush: BrushId<'b>, shape: &impl Shape) {
        self.scene.push(PaintCommand::Stroke {
            stroke: style.index,
            transform: self.transform,
            brush: brush.index,
            brush_transform: None,
            shape: PaintShape::from_shape(shape),
        });
    }
}

#[cfg(test)]
mod tests {
    use peniko::{
        Brush, Color, Fill,
        kurbo::{Affine, Rect},
    };

    use crate::{offset::Offset, paint::scene::PaintCommand};

    use super::Canvas;

    #[test]
    fn fill_records_a_command_at_the_current_transform() {
        let scene = Canvas::record(|canvas| {
            let white = canvas.brush(Color::WHITE);

            canvas.fill(Fill::NonZero, white, &Rect::new(0.0, 0.0, 10.0, 10.0));
        });

        assert_eq!(scene.len(), 1);
        match &scene.commands()[0] {
            PaintCommand::Fill {
                transform, brush, ..
            } => {
                assert_eq!(*transform, Affine::IDENTITY);

                assert!(matches!(scene.brush(*brush), Brush::Solid(c) if *c == Color::WHITE));
            }

            other => panic!("expected a fill, got {other:?}"),
        }
    }

    #[test]
    fn with_offset_stamps_translation_onto_drawing() {
        let scene = Canvas::record(|canvas| {
            canvas.with_offset(Offset::new(5.0_f32, 7.0_f32), |canvas| {
                let black = canvas.brush(Color::BLACK);

                canvas.fill(Fill::NonZero, black, &Rect::new(0.0, 0.0, 1.0, 1.0));
            });
        });

        let PaintCommand::Fill { transform, .. } = &scene.commands()[0] else {
            panic!("expected a fill");
        };

        assert_eq!(*transform, Affine::translate((5.0, 7.0)));
    }

    #[test]
    fn nested_offsets_compose_and_restore() {
        let scene = Canvas::record(|canvas| {
            canvas.with_offset(Offset::new(10.0_f32, 0.0_f32), |canvas| {
                canvas.with_offset(Offset::new(0.0_f32, 4.0_f32), |canvas| {
                    let white = canvas.brush(Color::WHITE);

                    canvas.fill(Fill::NonZero, white, &Rect::new(0.0, 0.0, 1.0, 1.0));
                });

                let white = canvas.brush(Color::WHITE);

                canvas.fill(Fill::NonZero, white, &Rect::new(0.0, 0.0, 1.0, 1.0));
            });
        });

        let PaintCommand::Fill {
            transform: inner, ..
        } = &scene.commands()[0]
        else {
            panic!("expected a fill");
        };
        let PaintCommand::Fill {
            transform: outer, ..
        } = &scene.commands()[1]
        else {
            panic!("expected a fill");
        };
        assert_eq!(*inner, Affine::translate((10.0, 4.0)));
        assert_eq!(*outer, Affine::translate((10.0, 0.0)));
    }

    #[test]
    fn with_clip_brackets_drawing_in_push_pop_layer() {
        let scene = Canvas::record(|canvas| {
            canvas.with_clip(&Rect::new(0.0, 0.0, 10.0, 10.0), |canvas| {
                let white = canvas.brush(Color::WHITE);

                canvas.fill(Fill::NonZero, white, &Rect::new(0.0, 0.0, 1.0, 1.0));
            });
        });

        assert_eq!(scene.len(), 3);
        assert!(matches!(
            scene.commands()[0],
            PaintCommand::PushLayer { .. }
        ));
        assert!(matches!(scene.commands()[1], PaintCommand::Fill { .. }));
        assert!(matches!(scene.commands()[2], PaintCommand::PopLayer));
    }
}

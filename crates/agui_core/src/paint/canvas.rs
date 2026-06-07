use std::marker::PhantomData;

use peniko::{
    BlendMode, Brush, Fill,
    kurbo::{Affine, Shape, Stroke},
};

use crate::{
    geometry::Offset,
    paint::{
        command::{GlyphInstance, PaintCommand, PaintShape},
        scene::Scene,
    },
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
/// strokes, and clips, which accumulate into a [`Scene`] instead of drawing immediately. Drawing is
/// recorded in local coordinates; [`with_transform`](Canvas::with_transform) brackets a region under
/// a transform. Obtain a canvas through [`Canvas::record`].
pub struct Canvas<'b> {
    scene: &'b mut Scene,
}

impl Canvas<'_> {
    /// Records a paint pass and returns the resulting [`Scene`].
    pub fn record(build: impl for<'b> FnOnce(&mut Canvas<'b>)) -> Scene {
        let mut scene = Scene::new();

        build(&mut Canvas { scene: &mut scene });

        scene
    }

    /// Records a paint pass into `scene`, clearing it first.
    pub fn record_into(scene: &mut Scene, build: impl for<'b> FnOnce(&mut Canvas<'b>)) {
        scene.reset();

        build(&mut Canvas { scene });
    }
}

impl<'b> Canvas<'b> {
    /// Records into `scene` directly. The caller owns the recording boundary.
    pub(crate) fn over(scene: &'b mut Scene) -> Self {
        Self { scene }
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

    /// Records the enclosed drawing under `transform`, concatenated onto the transform already in
    /// effect.
    pub fn with_transform(&mut self, transform: Affine, f: impl FnOnce(&mut Self)) {
        self.scene.push(PaintCommand::PushTransform(transform));

        f(self);

        self.scene.push(PaintCommand::PopTransform);
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
            brush: brush.index,
            brush_transform: None,
            shape: PaintShape::from_shape(shape),
        });
    }

    /// Draws `glyphs` from `font` at `font_size`, filled with `brush`. Glyph positions are in the
    /// coordinate system in effect.
    pub fn draw_glyphs(
        &mut self,
        font: &peniko::Font,
        font_size: f32,
        brush: BrushId<'b>,
        glyphs: Vec<GlyphInstance>,
    ) {
        self.scene.push(PaintCommand::DrawGlyphs {
            font: font.clone(),
            font_size,
            brush: brush.index,
            glyphs,
        });
    }

    /// Strokes the outline of `shape` with `brush`, using the registered `style`.
    pub fn stroke(&mut self, style: StrokeId<'b>, brush: BrushId<'b>, shape: &impl Shape) {
        self.scene.push(PaintCommand::Stroke {
            stroke: style.index,
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

    use crate::{
        geometry::Offset,
        paint::command::{GlyphInstance, PaintCommand},
    };

    use super::Canvas;

    #[test]
    fn draw_glyphs_records_a_draw_glyphs_command() {
        let scene = Canvas::record(|canvas| {
            let black = canvas.brush(Color::BLACK);
            let font = peniko::Font::new(peniko::Blob::new(std::sync::Arc::new(Vec::new())), 0);

            canvas.draw_glyphs(
                &font,
                20.0,
                black,
                vec![GlyphInstance {
                    id: 7,
                    x: 1.0,
                    y: 2.0,
                }],
            );
        });

        assert_eq!(scene.len(), 1);
        match &scene.commands()[0] {
            PaintCommand::DrawGlyphs { glyphs, brush, .. } => {
                assert_eq!(glyphs.len(), 1);
                assert_eq!(glyphs[0].id, 7);
                assert!(matches!(scene.brush(*brush), Brush::Solid(c) if *c == Color::BLACK));
            }

            other => panic!("expected draw glyphs, got {other:?}"),
        }
    }

    #[test]
    fn fill_records_a_fill_command() {
        let scene = Canvas::record(|canvas| {
            let white = canvas.brush(Color::WHITE);

            canvas.fill(Fill::NonZero, white, &Rect::new(0.0, 0.0, 10.0, 10.0));
        });

        assert_eq!(scene.len(), 1);
        match &scene.commands()[0] {
            PaintCommand::Fill { brush, .. } => {
                assert!(matches!(scene.brush(*brush), Brush::Solid(c) if *c == Color::WHITE));
            }

            other => panic!("expected a fill, got {other:?}"),
        }
    }

    #[test]
    fn with_offset_brackets_drawing_in_push_pop_transform() {
        let scene = Canvas::record(|canvas| {
            canvas.with_offset(Offset::new(5.0, 7.0), |canvas| {
                let black = canvas.brush(Color::BLACK);

                canvas.fill(Fill::NonZero, black, &Rect::new(0.0, 0.0, 1.0, 1.0));
            });
        });

        assert!(matches!(
            scene.commands()[0],
            PaintCommand::PushTransform(t) if t == Affine::translate((5.0, 7.0))
        ));
        assert!(matches!(scene.commands()[1], PaintCommand::Fill { .. }));
        assert!(matches!(scene.commands()[2], PaintCommand::PopTransform));
    }

    #[test]
    fn nested_offsets_nest_push_transforms() {
        let scene = Canvas::record(|canvas| {
            canvas.with_offset(Offset::new(10.0, 0.0), |canvas| {
                canvas.with_offset(Offset::new(0.0, 4.0), |canvas| {
                    let white = canvas.brush(Color::WHITE);

                    canvas.fill(Fill::NonZero, white, &Rect::new(0.0, 0.0, 1.0, 1.0));
                });
            });
        });

        assert!(matches!(
            scene.commands()[0],
            PaintCommand::PushTransform(t) if t == Affine::translate((10.0, 0.0))
        ));
        assert!(matches!(
            scene.commands()[1],
            PaintCommand::PushTransform(t) if t == Affine::translate((0.0, 4.0))
        ));
        assert!(matches!(scene.commands()[2], PaintCommand::Fill { .. }));
        assert!(matches!(scene.commands()[3], PaintCommand::PopTransform));
        assert!(matches!(scene.commands()[4], PaintCommand::PopTransform));
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

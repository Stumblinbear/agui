use std::rc::Rc;

use peniko::kurbo;

use crate::paint::{convert::DEFAULT_TOLERANCE, scene::Scene};

/// The geometry of a fill, stroke, or clip.
#[derive(Clone, Debug)]
pub enum PaintShape {
    Rect(kurbo::Rect),
    RoundedRect(kurbo::RoundedRect),
    Circle(kurbo::Circle),
    Path(kurbo::BezPath),
}

impl PaintShape {
    pub(crate) fn from_shape(shape: &impl kurbo::Shape) -> Self {
        if let Some(rect) = shape.as_rect() {
            Self::Rect(rect)
        } else if let Some(rounded) = shape.as_rounded_rect() {
            Self::RoundedRect(rounded)
        } else if let Some(circle) = shape.as_circle() {
            Self::Circle(circle)
        } else {
            Self::Path(shape.to_path(DEFAULT_TOLERANCE))
        }
    }
}

/// A glyph positioned within a [`PaintCommand::DrawGlyphs`] run, in the run's local coordinates.
///
/// Raw f32: a shaped text metric at the parley/vello seam, not a geometry value.
#[derive(Clone, Copy, Debug)]
pub struct GlyphInstance {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

/// A single operation within a [`Scene`].
#[derive(Clone, Debug)]
pub enum PaintCommand {
    /// Concatenates a transform onto the stack: every operation up to the matching [`PopTransform`]
    /// is placed under it.
    ///
    /// [`PopTransform`]: PaintCommand::PopTransform
    PushTransform(kurbo::Affine),

    /// Restores the transform in effect before the most recent [`PushTransform`].
    ///
    /// [`PushTransform`]: PaintCommand::PushTransform
    PopTransform,

    /// Begins a layer: every operation up to the matching [`PopLayer`] is clipped to `clip` and
    /// composited as one group, using `blend` and `alpha`.
    ///
    /// [`PopLayer`]: PaintCommand::PopLayer
    PushLayer {
        blend: peniko::BlendMode,
        alpha: f32,
        clip: PaintShape,
    },

    /// Closes the most recently opened layer.
    PopLayer,

    /// Fills a shape's interior.
    Fill {
        style: peniko::Fill,
        brush: u32,
        brush_transform: Option<Box<kurbo::Affine>>,
        shape: PaintShape,
    },

    /// Strokes a shape's outline.
    Stroke {
        stroke: u32,
        brush: u32,
        brush_transform: Option<Box<kurbo::Affine>>,
        shape: PaintShape,
    },

    /// Draws a run of positioned glyphs from one font at one size, filled with `brush`. Positions are
    /// baked into each glyph's coordinates; the transform in effect places the run.
    DrawGlyphs {
        font: peniko::Font,
        font_size: f32,
        brush: u32,
        glyphs: Vec<GlyphInstance>,
    },

    /// Splices a sub-scene under the transform in effect, by reference. The sub-scene keeps its own
    /// brush and stroke tables; a backend transforms it as a unit.
    Embed { scene: Rc<Scene> },
}

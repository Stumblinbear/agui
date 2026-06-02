use peniko::{
    BlendMode, Brush, Fill,
    kurbo::{self, Affine, BezPath, Shape, Stroke},
};

use crate::paint::convert::DEFAULT_TOLERANCE;

/// A complete, backend-independent description of what to draw.
///
/// A paint pass records into a scene through a [`Canvas`](crate::paint::Canvas); a rendering backend
/// then consumes the finished scene to produce pixels.
#[derive(Default, Clone, Debug)]
pub struct Scene {
    commands: Vec<PaintCommand>,

    brushes: Vec<Brush>,
    strokes: Vec<Stroke>,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    /// Discards all recorded operations, leaving the scene empty for reuse.
    pub fn reset(&mut self) {
        self.commands.clear();
        self.brushes.clear();
        self.strokes.clear();
    }

    pub fn commands(&self) -> &[PaintCommand] {
        &self.commands
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// The brush a command refers to by index.
    pub fn brush(&self, index: u32) -> &Brush {
        &self.brushes[index as usize]
    }

    /// The stroke style a [`PaintCommand::Stroke`] refers to by index.
    pub fn stroke(&self, index: u32) -> &Stroke {
        &self.strokes[index as usize]
    }

    pub(crate) fn push(&mut self, command: PaintCommand) {
        self.commands.push(command);
    }

    pub(crate) fn intern_brush(&mut self, brush: Brush) -> u32 {
        // Maybe don't register 4,294,967,295 brushes, nerd.
        #[allow(clippy::cast_possible_truncation)]
        let index = self.brushes.len() as u32;
        self.brushes.push(brush);
        index
    }

    pub(crate) fn intern_stroke(&mut self, stroke: Stroke) -> u32 {
        #[allow(clippy::cast_possible_truncation)]
        let index = self.strokes.len() as u32;
        self.strokes.push(stroke);
        index
    }
}

/// The geometry of a fill, stroke, or clip.
#[derive(Clone, Debug)]
pub enum PaintShape {
    Rect(kurbo::Rect),
    RoundedRect(kurbo::RoundedRect),
    Circle(kurbo::Circle),
    Path(BezPath),
}

impl PaintShape {
    pub(crate) fn from_shape(shape: &impl Shape) -> Self {
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

/// A single drawing operation within a [`Scene`].
#[derive(Clone, Debug)]
pub enum PaintCommand {
    /// Begins a layer: every operation up to the matching [`PaintCommand::PopLayer`] is clipped to
    /// `clip` and composited into the scene as one group, using `blend` and `alpha`.
    PushLayer {
        blend: BlendMode,
        alpha: f32,
        transform: Affine,
        clip: PaintShape,
    },
    /// Closes the most recently opened layer.
    PopLayer,
    /// Fills a shape's interior.
    Fill {
        style: Fill,
        transform: Affine,
        brush: u32,
        brush_transform: Option<Box<Affine>>,
        shape: PaintShape,
    },
    /// Strokes a shape's outline.
    Stroke {
        stroke: u32,
        transform: Affine,
        brush: u32,
        brush_transform: Option<Box<Affine>>,
        shape: PaintShape,
    },
}

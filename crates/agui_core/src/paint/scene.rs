use std::rc::Rc;

use peniko::{
    BlendMode, Brush, Fill,
    kurbo::{self, Affine, BezPath, Shape, Stroke},
};

use crate::paint::convert::DEFAULT_TOLERANCE;

/// A complete, backend-independent description of what to draw.
///
/// A paint pass records into a scene through a [`Canvas`](crate::paint::Canvas); a rendering backend
/// then consumes the finished scene to produce pixels
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

    /// Resolves this scene to one without [`Embed`](PaintCommand::Embed) references.
    ///
    /// A backend that can splice sub-scenes renders a scene with references directly; this is a
    /// fallback for backends that cannot, and a convenience for inspecting a result.
    pub fn flatten(&self) -> Scene {
        let mut out = Scene::new();
        self.inline_into(&mut out);
        out
    }

    fn inline_into(&self, out: &mut Scene) {
        for command in &self.commands {
            match command {
                PaintCommand::Embed { scene } => scene.inline_into(out),

                PaintCommand::Fill {
                    style,
                    brush,
                    brush_transform,
                    shape,
                } => {
                    let brush = out.intern_brush(self.brush(*brush).clone());

                    out.push(PaintCommand::Fill {
                        style: *style,
                        brush,
                        brush_transform: brush_transform.clone(),
                        shape: shape.clone(),
                    });
                }

                PaintCommand::Stroke {
                    stroke,
                    brush,
                    brush_transform,
                    shape,
                } => {
                    let stroke = out.intern_stroke(self.stroke(*stroke).clone());
                    let brush = out.intern_brush(self.brush(*brush).clone());

                    out.push(PaintCommand::Stroke {
                        stroke,
                        brush,
                        brush_transform: brush_transform.clone(),
                        shape: shape.clone(),
                    });
                }

                PaintCommand::PushTransform(transform) => {
                    out.push(PaintCommand::PushTransform(*transform));
                }

                PaintCommand::PopTransform => out.push(PaintCommand::PopTransform),

                PaintCommand::PushLayer { blend, alpha, clip } => {
                    out.push(PaintCommand::PushLayer {
                        blend: *blend,
                        alpha: *alpha,
                        clip: clip.clone(),
                    });
                }

                PaintCommand::PopLayer => out.push(PaintCommand::PopLayer),
            }
        }
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

/// A single operation within a [`Scene`].
#[derive(Clone, Debug)]
pub enum PaintCommand {
    /// Concatenates a transform onto the stack: every operation up to the matching [`PopTransform`]
    /// is placed under it.
    ///
    /// [`PopTransform`]: PaintCommand::PopTransform
    PushTransform(Affine),

    /// Restores the transform in effect before the most recent [`PushTransform`].
    ///
    /// [`PushTransform`]: PaintCommand::PushTransform
    PopTransform,

    /// Begins a layer: every operation up to the matching [`PopLayer`] is clipped to `clip` and
    /// composited as one group, using `blend` and `alpha`.
    ///
    /// [`PopLayer`]: PaintCommand::PopLayer
    PushLayer {
        blend: BlendMode,
        alpha: f32,
        clip: PaintShape,
    },

    /// Closes the most recently opened layer.
    PopLayer,

    /// Fills a shape's interior.
    Fill {
        style: Fill,
        brush: u32,
        brush_transform: Option<Box<Affine>>,
        shape: PaintShape,
    },

    /// Strokes a shape's outline.
    Stroke {
        stroke: u32,
        brush: u32,
        brush_transform: Option<Box<Affine>>,
        shape: PaintShape,
    },

    /// Splices a sub-scene under the transform in effect, by reference. The sub-scene keeps its own
    /// brush and stroke tables; a backend transforms it as a unit.
    Embed { scene: Rc<Scene> },
}

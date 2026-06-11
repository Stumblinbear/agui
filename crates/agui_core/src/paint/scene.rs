use peniko::{Brush, kurbo::Stroke};

use crate::paint::command::PaintCommand;

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

/// The buffer lengths of a recorded [`Scene`].
///
/// A caller that repeats a recording passes the lengths reported by the previous one as the
/// capacity for the next, so a recording of similar size fills pre-sized buffers instead of
/// growing them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SceneCapacity {
    commands: usize,
    brushes: usize,
    strokes: usize,
}

impl SceneCapacity {
    pub(crate) fn add(&mut self, other: SceneCapacity) {
        self.commands += other.commands;
        self.brushes += other.brushes;
        self.strokes += other.strokes;
    }

    pub(crate) fn saturating_sub(self, other: SceneCapacity) -> SceneCapacity {
        SceneCapacity {
            commands: self.commands.saturating_sub(other.commands),
            brushes: self.brushes.saturating_sub(other.brushes),
            strokes: self.strokes.saturating_sub(other.strokes),
        }
    }
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    /// The lengths of this scene's buffers, for sizing a later recording.
    pub fn lengths(&self) -> SceneCapacity {
        SceneCapacity {
            commands: self.commands.len(),
            brushes: self.brushes.len(),
            strokes: self.strokes.len(),
        }
    }

    /// Ensures each buffer can hold at least the corresponding count in `capacity` without
    /// reallocating.
    pub fn reserve(&mut self, capacity: SceneCapacity) {
        self.commands
            .reserve(capacity.commands.saturating_sub(self.commands.len()));
        self.brushes
            .reserve(capacity.brushes.saturating_sub(self.brushes.len()));
        self.strokes
            .reserve(capacity.strokes.saturating_sub(self.strokes.len()));
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

                PaintCommand::DrawGlyphs {
                    font,
                    font_size,
                    brush,
                    glyphs,
                } => {
                    let brush = out.intern_brush(self.brush(*brush).clone());

                    out.push(PaintCommand::DrawGlyphs {
                        font: font.clone(),
                        font_size: *font_size,
                        brush,
                        glyphs: glyphs.clone(),
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

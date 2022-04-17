use std::borrow::Cow;

use lyon::path::Path;

use crate::unit::{FontStyle, Rect, Shape, Size};

use self::{
    command::CanvasCommand,
    paint::{Brush, Paint},
};

pub mod command;
pub mod context;
pub mod paint;
pub mod renderer;
pub mod texture;

pub struct Canvas {
    size: Size,

    paint: Vec<Paint>,

    commands: Vec<CanvasCommand>,
}

impl std::hash::Hash for Canvas {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.size.hash(state);
        self.paint.hash(state);
        self.commands.hash(state);
    }
}

impl Canvas {
    pub fn new(size: Size) -> Self {
        Self {
            size,

            paint: Vec::default(),

            commands: Vec::default(),
        }
    }

    pub fn get_size(&self) -> Size {
        self.size
    }

    pub fn get_paints(&self) -> &Vec<Paint> {
        &self.paint
    }

    pub fn get_paint(&self, brush: Brush) -> &Paint {
        &self.paint[brush.idx()]
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn consume(&mut self) -> Option<CanvasCommand> {
        if self.commands.is_empty() {
            None
        } else {
            Some(self.commands.remove(0))
        }
    }

    pub fn new_brush(&mut self, paint: Paint) -> Brush {
        self.paint.push(paint);

        Brush::from(self.paint.len() - 1)
    }

    /// Starts a new layer with `shape`. It will be the `rect` of the canvas.
    pub fn start_layer(&mut self, brush: Brush, shape: Shape) {
        self.start_layer_at(self.size.into(), brush, shape);
    }

    /// Starts a new layer in the defined `rect` with `shape`.
    pub fn start_layer_at(&mut self, rect: Rect, brush: Brush, shape: Shape) {
        tracing::trace!("starting new layer");

        self.commands
            .push(CanvasCommand::Layer { rect, shape, brush });
    }

    /// Pop the last layer of the canvas.
    pub fn pop(&mut self) {
        tracing::trace!("popped layer");

        self.commands.push(CanvasCommand::Pop);
    }

    /// Draws a rectangle. It will be the `rect` of the canvas.
    pub fn draw_rect(&mut self, brush: Brush) {
        self.draw_rect_at(self.size.into(), brush);
    }

    /// Draws a rectangle in the defined `rect`.
    pub fn draw_rect_at(&mut self, rect: Rect, brush: Brush) {
        tracing::trace!("drawing rect");

        self.commands.push(CanvasCommand::Shape {
            rect,
            shape: Shape::Rect,

            brush,
        });
    }

    /// Draws a rounded rectangle. It will be the `rect` of the canvas.
    pub fn draw_rounded_rect(
        &mut self,
        brush: Brush,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) {
        self.draw_rounded_rect_at(
            self.size.into(),
            brush,
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        );
    }

    /// Draws a rounded rectangle in the defined `rect`.
    pub fn draw_rounded_rect_at(
        &mut self,
        rect: Rect,
        brush: Brush,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) {
        tracing::trace!("drawing rounded rect");

        self.commands.push(CanvasCommand::Shape {
            brush,
            rect,

            shape: Shape::RoundedRect {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            },
        });
    }

    /// Draws a path. It will be the `rect` of the canvas.
    pub fn draw_path(&mut self, brush: Brush, path: Path) {
        self.draw_path_at(self.size.into(), brush, path);
    }

    /// Draws a path in the defined `rect`.
    pub fn draw_path_at(&mut self, rect: Rect, brush: Brush, path: Path) {
        tracing::trace!("drawing path");

        self.commands.push(CanvasCommand::Shape {
            rect,
            brush,

            shape: Shape::Path(path),
        });
    }

    /// Draws text on the canvas. It will be wrapped to the `rect` of the canvas.
    pub fn draw_text(&mut self, brush: Brush, font: FontStyle, text: Cow<'static, str>) {
        self.draw_text_at(self.size.into(), brush, font, text);
    }

    /// Draws text on the canvas, ensuring it remains within the `rect`.
    pub fn draw_text_at(
        &mut self,
        rect: Rect,
        brush: Brush,
        font: FontStyle,
        text: Cow<'static, str>,
    ) {
        tracing::trace!("drawing text");

        self.commands.push(CanvasCommand::Text {
            rect,
            brush,

            font,

            text,
        });
    }
}

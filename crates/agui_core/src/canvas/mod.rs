use std::borrow::Cow;

use lyon::path::Path;

use crate::unit::{FontStyle, Rect, Shape, Size};

use self::{command::CanvasCommand, paint::Paint};

pub mod command;
pub mod context;
pub mod paint;
pub mod renderer;
pub mod texture;

pub struct Canvas {
    size: Size,

    commands: Vec<CanvasCommand>,

    depth: usize,
}

// Draw functions
impl Canvas {
    pub fn new(size: Size) -> Self {
        Self {
            size,

            commands: Vec::default(),

            depth: 0,
        }
    }

    pub fn get_size(&self) -> Size {
        self.size
    }

    pub fn consume(self) -> Vec<CanvasCommand> {
        self.commands
    }

    /// Starts a new layer with `shape`. It will be the `rect` of the canvas.
    pub fn start_layer(&mut self, paint: &Paint, shape: Shape) {
        self.start_layer_at(self.size.into(), paint, shape);
    }

    /// Starts a new layer in the defined `rect` with `shape`.
    pub fn start_layer_at(&mut self, rect: Rect, paint: &Paint, shape: Shape) {
        tracing::trace!(depth = self.depth, "starting new layer");

        self.depth += 1;

        self.commands.push(CanvasCommand::Layer {
            rect,
            shape,

            anti_alias: paint.anti_alias,
            blend_mode: paint.blend_mode,
        });
    }

    /// Pop the last layer of the canvas.
    pub fn pop(&mut self) {
        if self.depth == 0 {
            panic!("popping beyond the layers created by the canvas is not permitted");
        }

        tracing::trace!(depth = self.depth, "popping layer");

        self.depth -= 1;

        self.commands.push(CanvasCommand::Pop);
    }

    /// Draws a rectangle. It will be the `rect` of the canvas.
    pub fn draw_rect(&mut self, paint: &Paint) {
        self.draw_rect_at(self.size.into(), paint);
    }

    /// Draws a rectangle in the defined `rect`.
    pub fn draw_rect_at(&mut self, rect: Rect, paint: &Paint) {
        tracing::trace!(depth = self.depth, "drawing rect");

        self.commands.push(CanvasCommand::Shape {
            rect,
            shape: Shape::Rect,

            color: paint.color,
        });
    }

    /// Draws a rounded rectangle. It will be the `rect` of the canvas.
    pub fn draw_rounded_rect(
        &mut self,
        paint: &Paint,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) {
        self.draw_rounded_rect_at(
            self.size.into(),
            paint,
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
        paint: &Paint,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) {
        tracing::trace!(depth = self.depth, "drawing rounded rect");

        self.commands.push(CanvasCommand::Shape {
            rect,
            shape: Shape::RoundedRect {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            },

            color: paint.color,
        });
    }

    /// Draws a path. It will be the `rect` of the canvas.
    pub fn draw_path(&mut self, paint: &Paint, path: Path) {
        self.draw_path_at(self.size.into(), paint, path);
    }

    /// Draws a path in the defined `rect`.
    pub fn draw_path_at(&mut self, rect: Rect, paint: &Paint, path: Path) {
        tracing::trace!(depth = self.depth, "drawing path");

        self.commands.push(CanvasCommand::Shape {
            rect,
            shape: Shape::Path(path),

            color: paint.color,
        });
    }

    /// Draws text on the canvas. It will be wrapped to the `rect` of the canvas.
    pub fn draw_text(&mut self, paint: &Paint, font: FontStyle, text: Cow<'static, str>) {
        self.draw_text_at(self.size.into(), paint, font, text);
    }

    /// Draws text on the canvas, ensuring it remains within the `rect`.
    pub fn draw_text_at(
        &mut self,
        rect: Rect,
        paint: &Paint,
        font: FontStyle,
        text: Cow<'static, str>,
    ) {
        tracing::trace!(depth = self.depth, "drawing text");

        self.commands.push(CanvasCommand::Text {
            rect,

            color: paint.color,

            font,
            text,
        });
    }
}

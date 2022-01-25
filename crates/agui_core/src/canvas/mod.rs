use std::borrow::Cow;

use lyon::path::Path;

use crate::unit::{Rect, Shape};

use self::{
    clipping::Clip,
    command::CanvasCommand,
    font::FontStyle,
    paint::{Brush, Paint},
};

pub mod clipping;
pub mod command;
pub mod font;
pub mod paint;
pub mod painter;
pub mod texture;

#[derive(PartialEq)]
pub struct Canvas {
    rect: Rect,

    paint: Vec<Paint>,

    commands: Vec<CanvasCommand>,
}

impl Canvas {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,

            paint: Vec::default(),

            commands: Vec::default(),
        }
    }

    pub fn get_rect(&self) -> Rect {
        self.rect
    }

    pub fn get_paint(&self, brush: Brush) -> &Paint {
        &self.paint[brush.idx()]
    }

    pub fn get_commands(&self) -> &Vec<CanvasCommand> {
        &self.commands
    }

    pub fn new_brush(&mut self, paint: Paint) -> Brush {
        self.paint.push(paint);

        Brush::from(self.paint.len() - 1)
    }

    /// Begins clipping. It will be the `rect` of the canvas.
    pub fn start_clipping(&mut self, clip: Clip, shape: Shape) {
        self.start_clipping_at(self.rect, clip, shape);
    }

    /// Begins clipping the defined `rect`.
    pub fn start_clipping_at(&mut self, rect: Rect, clip: Clip, shape: Shape) {
        self.commands
            .push(CanvasCommand::Clip { rect, clip, shape });
    }

    /// Draws a rectangle. It will be the `rect` of the canvas.
    pub fn draw_rect(&mut self, brush: Brush) {
        self.draw_rect_at(self.rect, brush);
    }

    /// Draws a rectangle in the defined `rect`.
    pub fn draw_rect_at(&mut self, rect: Rect, brush: Brush) {
        self.commands.push(CanvasCommand::Shape {
            rect,
            brush,

            shape: Shape::Rect,
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
            self.rect,
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
        self.draw_path_at(self.rect, brush, path);
    }

    /// Draws a path in the defined `rect`.
    pub fn draw_path_at(&mut self, rect: Rect, brush: Brush, path: Path) {
        self.commands.push(CanvasCommand::Shape {
            rect,
            brush,

            shape: Shape::Path(path),
        });
    }

    /// Draws text on the canvas. It will be wrapped to the `rect` of the canvas.
    pub fn draw_text(&mut self, brush: Brush, font: FontStyle, text: Cow<'static, str>) {
        self.draw_text_at(self.rect, brush, font, text);
    }

    /// Draws text on the canvas, ensuring it remains within the `rect`.
    pub fn draw_text_at(&mut self, rect: Rect, brush: Brush, font: FontStyle, text: Cow<'static, str>) {
        self.commands.push(CanvasCommand::Text {
            rect,
            brush,

            font,

            text,
        });
    }
}

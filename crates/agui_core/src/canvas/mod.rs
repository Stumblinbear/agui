use lyon::path::Path;

use crate::unit::{Bounds, Color, Shape, Size};

use self::{command::CanvasCommand, font::FontDescriptor};

pub mod command;
pub mod font;
pub mod painter;

const BOUNDS_FULL: Bounds = Bounds {
    top: 0.0,
    right: 0.0,
    bottom: 0.0,
    left: 0.0,
};

#[derive(PartialEq)]
pub struct Canvas {
    size: Size,

    commands: Vec<CanvasCommand>,
}

impl Canvas {
    pub fn new(size: Size) -> Self {
        Self {
            size,

            commands: Vec::default(),
        }
    }

    pub fn get_size(&self) -> Size {
        self.size
    }

    pub fn get_commands(&self) -> &Vec<CanvasCommand> {
        &self.commands
    }

    fn validate_bounds(&self, bounds: Bounds) {
        if bounds.top < 0.0 || bounds.right < 0.0 || bounds.bottom < 0.0 || bounds.left < 0.0 {
            panic!(
                "cannot draw outside of canvas bounds: (0.0, 0.0) >= {:?}",
                bounds
            );
        }

        if bounds.top > self.size.height
            || bounds.right > self.size.width
            || bounds.bottom > self.size.height
            || bounds.left > self.size.width
        {
            panic!(
                "cannot draw outside of canvas bounds: ({:.2}, {:.2}) >= {:?}",
                self.size.width, self.size.height, bounds
            );
        }
    }

    pub fn draw_rect(&mut self, color: Color) {
        self.draw_rect_at(BOUNDS_FULL, color);
    }

    /// # Panics
    ///
    /// Will panic if you attempt to draw outside of the canvas bounds.
    pub fn draw_rect_at(&mut self, bounds: Bounds, color: Color) {
        self.validate_bounds(bounds);

        self.commands.push(CanvasCommand::Shape {
            bounds,

            color,
            shape: Shape::Rect,
        });
    }

    pub fn draw_rounded_rect(
        &mut self,
        color: Color,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) {
        self.draw_rounded_rect_at(
            color,
            BOUNDS_FULL,
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        );
    }

    /// # Panics
    ///
    /// Will panic if you attempt to draw outside of the canvas bounds.
    pub fn draw_rounded_rect_at(
        &mut self,
        color: Color,
        bounds: Bounds,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) {
        self.validate_bounds(bounds);

        self.commands.push(CanvasCommand::Shape {
            bounds,

            color,
            shape: Shape::RoundedRect {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            },
        });
    }

    pub fn draw_path(&mut self, color: Color, path: Path) {
        self.draw_path_at(BOUNDS_FULL, color, path);
    }

    /// # Panics
    ///
    /// Will panic if you attempt to draw outside of the canvas bounds.
    pub fn draw_path_at(&mut self, bounds: Bounds, color: Color, path: Path) {
        self.validate_bounds(bounds);

        self.commands.push(CanvasCommand::Shape {
            bounds,

            shape: Shape::Path(path),

            color,
        });
    }

    pub fn draw_text(&mut self, bounds: Bounds, color: Color, font: FontDescriptor, text: String) {
        self.validate_bounds(bounds);

        self.commands.push(CanvasCommand::Text {
            bounds,

            font,
            text,

            color,
        });
    }
}

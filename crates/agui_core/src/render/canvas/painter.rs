use std::borrow::Cow;

use lyon::path::Path;

use crate::{
    render::canvas::CanvasLayer,
    unit::{FontStyle, Rect, Shape, Size},
};

use super::{command::CanvasCommand, paint::Paint, Canvas, CanvasStyle};

pub struct CanvasPainter<'paint> {
    canvas: &'paint mut Canvas,
}

impl<'paint> CanvasPainter<'paint> {
    pub fn new(canvas: &'paint mut Canvas) -> CanvasPainter<'paint> {
        CanvasPainter { canvas }
    }

    pub fn get_size(&self) -> Size {
        self.canvas.rect.into()
    }

    fn push_command(&mut self, command: CanvasCommand) {
        // If we have children, but a new layer has not been started afterwards, panic
        if !self.canvas.children.is_empty() {
            panic!("cannot start drawing on an uninitialized layer");
        }

        self.canvas.head.push(command);
    }

    /// Starts a layer with `shape` which child widgets will drawn to. It will be the `rect` of the canvas.
    pub fn start_layer(self, paint: &Paint, shape: Shape) -> CanvasPainter<'paint> {
        let rect = self.canvas.rect;

        self.start_layer_at(rect, paint, shape)
    }

    /// Starts a layer in the defined `rect` with `shape` which child widgets will drawn to.
    pub fn start_layer_at(self, rect: Rect, paint: &Paint, shape: Shape) -> CanvasPainter<'paint> {
        tracing::trace!("starting new layer");

        self.canvas.tail = Some(Box::new(CanvasLayer {
            style: CanvasStyle {
                shape,

                anti_alias: paint.anti_alias,
                blend_mode: paint.blend_mode,
            },

            canvas: Canvas {
                rect,
                ..Canvas::default()
            },
        }));

        CanvasPainter::new(&mut self.canvas.tail.as_mut().unwrap().canvas)
    }

    /// Creates a layer with `shape`. It will be the `rect` of the canvas.
    pub fn layer(&mut self, paint: &Paint, shape: Shape, func: impl FnOnce(&mut CanvasPainter)) {
        self.layer_at(self.canvas.rect, paint, shape, func);
    }

    /// Creates a layer with `shape`. It will be the `rect` of the canvas.
    pub fn layer_at(
        &mut self,
        rect: Rect,
        paint: &Paint,
        shape: Shape,
        func: impl FnOnce(&mut CanvasPainter),
    ) {
        tracing::trace!("creating new layer");

        self.canvas.children.push(CanvasLayer {
            style: CanvasStyle {
                shape,

                anti_alias: paint.anti_alias,
                blend_mode: paint.blend_mode,
            },

            canvas: Canvas {
                rect,
                ..Canvas::default()
            },
        });

        func(&mut CanvasPainter::new(
            &mut self.canvas.children.last_mut().unwrap().canvas,
        ));
    }

    /// Draws a rectangle. It will be the `rect` of the canvas.
    pub fn draw_rect(&mut self, paint: &Paint) {
        self.draw_rect_at(self.canvas.rect, paint);
    }

    /// Draws a rectangle in the defined `rect`.
    pub fn draw_rect_at(&mut self, rect: Rect, paint: &Paint) {
        tracing::trace!("drawing rect");

        self.push_command(CanvasCommand::Shape {
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
            self.canvas.rect,
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
        tracing::trace!("drawing rounded rect");

        self.push_command(CanvasCommand::Shape {
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
        self.draw_path_at(self.canvas.rect, paint, path);
    }

    /// Draws a path in the defined `rect`.
    pub fn draw_path_at(&mut self, rect: Rect, paint: &Paint, path: Path) {
        tracing::trace!("drawing path");

        self.push_command(CanvasCommand::Shape {
            rect,
            shape: Shape::Path(path),

            color: paint.color,
        });
    }

    /// Draws text on the canvas. It will be wrapped to the `rect` of the canvas.
    pub fn draw_text(&mut self, paint: &Paint, font: FontStyle, text: Cow<'static, str>) {
        self.draw_text_at(self.canvas.rect, paint, font, text);
    }

    /// Draws text on the canvas, ensuring it remains within the `rect`.
    pub fn draw_text_at(
        &mut self,
        rect: Rect,
        paint: &Paint,
        font: FontStyle,
        text: Cow<'static, str>,
    ) {
        tracing::trace!("drawing text");

        self.push_command(CanvasCommand::Text {
            rect,

            color: paint.color,

            font,
            text,
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn canvas_style() {}
}

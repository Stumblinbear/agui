use std::borrow::Cow;

use lyon::path::Path;

use crate::unit::{FontStyle, Rect, Shape, Size};

use self::{command::CanvasCommand, element::RenderElement, layer::LayerStyle, paint::Paint};

pub mod command;
pub mod context;
pub mod element;
pub mod layer;
pub mod paint;
pub mod renderer;
pub mod texture;

pub struct Canvas {
    pub(crate) rect: Rect,

    pub(crate) layer_style: Option<LayerStyle>,

    pub(crate) head: Option<RenderElement>,
    pub(crate) children: Vec<Canvas>,
    pub(crate) tail: Vec<Canvas>,
}

impl Canvas {
    pub fn get_size(&self) -> Size {
        self.rect.into()
    }

    fn push_command(&mut self, command: CanvasCommand) {
        // If we have children, but a new layer has not been started afterwards, panic
        if !self.children.is_empty() && self.tail.is_empty() {
            panic!("cannot start drawing on an uninitialized layer");
        }

        if let Some(tail) = self.tail.last_mut() {
            tail.push_command(command);
        } else {
            self.head
                .get_or_insert_with(RenderElement::default)
                .commands
                .push(command);
        }
    }

    /// Starts a layer with `shape` which child widgets will drawn to. It will be the `rect` of the canvas.
    pub fn start_layer(&mut self, paint: &Paint, shape: Shape) {
        self.start_layer_at(self.rect, paint, shape);
    }

    /// Starts a layer in the defined `rect` with `shape` which child widgets will drawn to.
    pub fn start_layer_at(&mut self, rect: Rect, paint: &Paint, shape: Shape) {
        tracing::trace!("starting new layer");

        self.tail.push(Canvas {
            rect,

            layer_style: Some(LayerStyle {
                shape,

                anti_alias: paint.anti_alias,
                blend_mode: paint.blend_mode,
            }),

            head: None,
            children: Vec::default(),
            tail: Vec::default(),
        });
    }

    /// Creates a layer with `shape`. It will be the `rect` of the canvas.
    pub fn layer(&mut self, paint: &Paint, shape: Shape, func: impl FnOnce(&mut Canvas)) {
        self.layer_at(self.rect, paint, shape, func);
    }

    /// Creates a layer with `shape`. It will be the `rect` of the canvas.
    pub fn layer_at(
        &mut self,
        rect: Rect,
        paint: &Paint,
        shape: Shape,
        func: impl FnOnce(&mut Canvas),
    ) {
        if let Some(tail) = self.tail.last_mut() {
            return tail.layer_at(rect, paint, shape, func);
        }

        tracing::trace!("creating new layer");

        self.children.push(Canvas {
            rect: self.rect,

            layer_style: Some(LayerStyle {
                shape,

                anti_alias: paint.anti_alias,
                blend_mode: paint.blend_mode,
            }),
            head: None,
            children: Vec::default(),
            tail: Vec::default(),
        });

        let child = self.children.last_mut().unwrap();

        func(child);
    }

    /// Draws a rectangle. It will be the `rect` of the canvas.
    pub fn draw_rect(&mut self, paint: &Paint) {
        self.draw_rect_at(self.rect, paint);
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
            self.rect,
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
        self.draw_path_at(self.rect, paint, path);
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
        self.draw_text_at(self.rect, paint, font, text);
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

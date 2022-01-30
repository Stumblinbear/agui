use std::borrow::Cow;

use lyon::path::Path;

use crate::unit::{Rect, Shape, Size};

use self::{
    clipping::Clip,
    command::{CanvasCommand, TextListenerId},
    font::FontStyle,
    paint::{Brush, Paint},
};

pub mod clipping;
pub mod command;
pub mod font;
pub mod paint;
pub mod renderer;
pub mod texture;

pub struct Canvas {
    // The insertion cursor. Currently only used for text size listeners.
    cursor: usize,

    size: Size,

    paint: Vec<Paint>,

    commands: Vec<CanvasCommand>,

    text_listener: Vec<Option<Box<dyn FnOnce(&mut Canvas, Size)>>>,
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
            cursor: 0,

            size,

            paint: Vec::default(),

            commands: Vec::default(),

            text_listener: Vec::default(),
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

    #[allow(clippy::borrowed_box)]
    pub fn resolve_text_listener(&mut self, id: TextListenerId, text_size: Size) {
        let listener_func = self.text_listener[id.0]
            .take()
            .expect("text listener already resolved");

        // Text listeners will only be resolved when they're at the position zero.
        self.cursor = 0;

        listener_func(self, text_size);

        // Once the new commands have been resolved, reset the cursor to the end.
        self.cursor = self.commands.len();
    }

    fn add_command(&mut self, command: CanvasCommand) {
        self.commands.insert(self.cursor, command);

        self.cursor += 1;
    }

    pub fn new_brush(&mut self, paint: Paint) -> Brush {
        self.paint.push(paint);

        Brush::from(self.paint.len() - 1)
    }

    /// Begins clipping. It will be the `rect` of the canvas.
    pub fn start_clipping(&mut self, clip: Clip, shape: Shape) {
        self.start_clipping_at(self.size.into(), clip, shape);
    }

    /// Begins clipping the defined `rect`.
    pub fn start_clipping_at(&mut self, rect: Rect, clip: Clip, shape: Shape) {
        self.add_command(CanvasCommand::Clip { rect, clip, shape });
    }

    /// Draws a rectangle. It will be the `rect` of the canvas.
    pub fn draw_rect(&mut self, brush: Brush) {
        self.draw_rect_at(self.size.into(), brush);
    }

    /// Draws a rectangle in the defined `rect`.
    pub fn draw_rect_at(&mut self, rect: Rect, brush: Brush) {
        self.add_command(CanvasCommand::Shape {
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
        self.add_command(CanvasCommand::Shape {
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
        self.add_command(CanvasCommand::Shape {
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
        self.add_command(CanvasCommand::Text {
            rect,
            brush,

            font,

            text,
        });
    }

    /// Calculate the size of text on the canvas. It will be wrapped to the `rect` of the canvas.
    pub fn calc_text_size<F>(&mut self, font: FontStyle, text: Cow<'static, str>, func: F)
    where
        F: FnOnce(&mut Self, Size) + 'static,
    {
        self.calc_text_size_for(self.size, font, text, func);
    }

    /// Calculate the size of text on the canvas. It will be wrapped to the given `size`.
    pub fn calc_text_size_for<F>(
        &mut self,
        size: Size,
        font: FontStyle,
        text: Cow<'static, str>,
        func: F,
    ) where
        F: FnOnce(&mut Self, Size) + 'static,
    {
        self.text_listener.push(Some(Box::new(func)));

        self.add_command(CanvasCommand::TextListener {
            size,

            font,

            text,

            id: TextListenerId(self.text_listener.len() - 1),
        });
    }
}

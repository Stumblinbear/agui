use std::{borrow::Cow, marker::PhantomData};

use lyon::path::Path;

use crate::{
    render::{
        canvas::{command::CanvasCommand, Canvas, CanvasLayer, LayerStyle},
        paint::Paint,
    },
    unit::{FontStyle, Rect, Shape, Size},
};

pub trait CanvasPainterState {}

pub struct Head;
pub struct Tail;

impl CanvasPainterState for Head {}
impl CanvasPainterState for Tail {}

pub struct CanvasPainter<'paint, State>
where
    State: CanvasPainterState,
{
    phantom: PhantomData<State>,

    canvas: &'paint mut Canvas,
}

impl<'paint, State> CanvasPainter<'paint, State>
where
    State: CanvasPainterState,
{
    pub(crate) fn begin(canvas: &'paint mut Canvas) -> CanvasPainter<'paint, State> {
        CanvasPainter {
            phantom: PhantomData,

            canvas,
        }
    }

    pub fn get_size(&self) -> Size {
        self.canvas.size
    }

    fn push_command(&mut self, command: CanvasCommand) {
        // If we have children, but a new layer has not been started afterwards, panic
        if !self.canvas.children.is_empty() {
            panic!("cannot start drawing on an uninitialized layer");
        }

        self.canvas.head.push(command);
    }

    /// Starts a layer with `shape` which child widgets will drawn to. It will be the `rect` of the canvas.
    pub fn start_layer(self, paint: &Paint, shape: Shape) -> CanvasPainter<'paint, Head> {
        let rect = self.canvas.size.into();

        self.start_layer_at(rect, paint, shape)
    }

    /// Starts a layer in the defined `rect` with `shape` which child widgets will drawn to.
    pub fn start_layer_at(
        self,
        rect: Rect,
        paint: &Paint,
        shape: Shape,
    ) -> CanvasPainter<'paint, Head> {
        tracing::trace!("starting new layer");

        self.canvas.tail = Some(Box::new(CanvasLayer {
            offset: rect.into(),

            style: LayerStyle {
                shape,

                anti_alias: paint.anti_alias,
                blend_mode: paint.blend_mode,
            },

            canvas: Canvas {
                size: rect.into(),

                head: Vec::default(),
                children: Vec::default(),
                tail: None,
            },
        }));

        CanvasPainter::<Head>::begin(&mut self.canvas.tail.as_mut().unwrap().canvas)
    }

    /// Creates a layer with `shape`. It will be the `rect` of the canvas.
    pub fn layer(
        self,
        paint: &Paint,
        shape: Shape,
        func: impl FnOnce(&mut CanvasPainter<Head>),
    ) -> CanvasPainter<'paint, Tail> {
        let rect = self.canvas.size.into();

        self.layer_at(rect, paint, shape, func)
    }

    /// Creates a layer with `shape`. It will be the `rect` of the canvas.
    pub fn layer_at(
        self,
        rect: Rect,
        paint: &Paint,
        shape: Shape,
        func: impl FnOnce(&mut CanvasPainter<Head>),
    ) -> CanvasPainter<'paint, Tail> {
        tracing::trace!("creating new layer");

        self.canvas.children.push(CanvasLayer {
            offset: rect.into(),

            style: LayerStyle {
                shape,

                anti_alias: paint.anti_alias,
                blend_mode: paint.blend_mode,
            },

            canvas: Canvas {
                size: rect.into(),

                head: Vec::default(),
                children: Vec::default(),
                tail: None,
            },
        });

        func(&mut CanvasPainter {
            phantom: PhantomData,

            canvas: &mut self.canvas.children.last_mut().unwrap().canvas,
        });

        CanvasPainter::<Tail>::begin(self.canvas)
    }
}

impl<'paint> CanvasPainter<'paint, Head> {
    /// Draws a rectangle. It will be the `rect` of the canvas.
    pub fn draw_rect(&mut self, paint: &Paint) {
        self.draw_rect_at(self.canvas.size.into(), paint);
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
            self.canvas.size.into(),
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
        self.draw_path_at(self.canvas.size.into(), paint, path);
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
    pub fn draw_text<T>(&mut self, paint: &Paint, font: FontStyle, text: T)
    where
        T: Into<Cow<'static, str>>,
    {
        self.draw_text_at(self.canvas.size.into(), paint, font, text);
    }

    /// Draws text on the canvas, ensuring it remains within the `rect`.
    pub fn draw_text_at<T>(&mut self, rect: Rect, paint: &Paint, font: FontStyle, text: T)
    where
        T: Into<Cow<'static, str>>,
    {
        tracing::trace!("drawing text");

        self.push_command(CanvasCommand::Text {
            rect,

            color: paint.color,

            font,
            text: text.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn canvas_style() {}
}

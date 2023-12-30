use std::{borrow::Cow, marker::PhantomData};

use lyon::path::Path;

use crate::{
    render::{
        canvas::{command::CanvasCommand, Canvas, CanvasLayer, LayerStyle},
        paint::Paint,
        Brush,
    },
    unit::{Rect, Shape, Size, TextStyle},
};

pub trait CanvasPainterState {}

pub struct Head<T> {
    phantom: PhantomData<T>,
}

pub struct Layer<T> {
    phantom: PhantomData<T>,
}

pub struct Tail<T> {
    phantom: PhantomData<T>,
}

impl CanvasPainterState for () {}
impl<T> CanvasPainterState for Head<T> where T: CanvasPainterState {}
impl<T> CanvasPainterState for Layer<T> where T: CanvasPainterState {}
impl<T> CanvasPainterState for Tail<T> where T: CanvasPainterState {}

pub struct CanvasPainter<'paint, State = Head<()>>
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
    pub fn begin(canvas: &'paint mut Canvas) -> CanvasPainter<'paint, State> {
        CanvasPainter {
            phantom: PhantomData,

            canvas,
        }
    }

    pub fn size(&self) -> Size {
        self.canvas.size
    }

    // TODO: add a lifetime to brushes so they cannot be used outside of the canvas or layer they belong to
    pub fn add_paint(&mut self, paint: Paint) -> Brush<State> {
        self.canvas.paints.push(paint);

        Brush {
            phantom: PhantomData,

            idx: self.canvas.paints.len() - 1,
        }
    }

    fn push_command(&mut self, command: CanvasCommand) {
        // If we have children, but a new layer has not been started afterwards, panic
        if !self.canvas.children.is_empty() {
            panic!("cannot start drawing on an uninitialized layer");
        }

        self.canvas.head.push(command);
    }

    /// Creates a layer with `shape`. It will be the `rect` of the canvas.
    pub fn layer<F>(
        self,
        brush: &Brush<State>,
        shape: Shape,
        func: F,
    ) -> CanvasPainter<'paint, Tail<State>>
    where
        F: for<'layer> FnOnce(CanvasPainter<'layer, Head<State>>),
    {
        let rect = self.canvas.size.into();

        self.layer_at(rect, brush, shape, func)
    }

    /// Creates a layer with `shape`. It will be the `rect` of the canvas.
    pub fn layer_at<F>(
        self,
        rect: Rect,
        brush: &Brush<State>,
        shape: Shape,
        func: F,
    ) -> CanvasPainter<'paint, Tail<State>>
    where
        F: for<'layer> FnOnce(CanvasPainter<'layer, Head<State>>),
    {
        tracing::trace!("creating new layer");

        self.canvas.children.push(CanvasLayer {
            offset: rect.into(),

            style: LayerStyle {
                paint_idx: brush.idx(),

                shape,
            },

            canvas: Canvas {
                size: rect.into(),

                paints: Vec::default(),

                head: Vec::default(),
                children: Vec::default(),
                tail: None,
            },
        });

        func(CanvasPainter {
            phantom: PhantomData,

            canvas: &mut self.canvas.children.last_mut().unwrap().canvas,
        });

        CanvasPainter::<Tail<State>>::begin(self.canvas)
    }

    /// Starts a layer with `shape` which child widgets will drawn to. It will be the `rect` of the canvas.
    pub fn start_layer(
        self,
        brush: &Brush<State>,
        shape: Shape,
    ) -> CanvasPainter<'paint, Head<State>> {
        let rect = self.canvas.size.into();

        self.start_layer_at(rect, brush, shape)
    }

    /// Starts a layer in the defined `rect` with `shape` which child widgets will drawn to.
    pub fn start_layer_at(
        self,
        rect: Rect,
        brush: &Brush<State>,
        shape: Shape,
    ) -> CanvasPainter<'paint, Head<State>> {
        tracing::trace!("starting new layer");

        self.canvas.tail = Some(Box::new(CanvasLayer {
            offset: rect.into(),

            style: LayerStyle {
                paint_idx: brush.idx(),

                shape,
            },

            canvas: Canvas {
                size: rect.into(),

                paints: Vec::default(),

                head: Vec::default(),
                children: Vec::default(),
                tail: None,
            },
        }));

        CanvasPainter::<Head<State>>::begin(&mut self.canvas.tail.as_mut().unwrap().canvas)
    }
}

impl<'paint, State> CanvasPainter<'paint, Head<State>>
where
    State: CanvasPainterState,
{
    /// Draws a rectangle. It will be the `rect` of the canvas.
    pub fn draw_rect(&mut self, brush: &Brush<Head<State>>) {
        self.draw_rect_at(self.canvas.size.into(), brush);
    }

    /// Draws a rectangle in the defined `rect`.
    pub fn draw_rect_at(&mut self, rect: Rect, brush: &Brush<Head<State>>) {
        tracing::trace!("drawing rect");

        self.push_command(CanvasCommand::Shape {
            paint_idx: brush.idx(),

            rect,
            shape: Shape::Rect,
        });
    }

    /// Draws a rounded rectangle. It will be the `rect` of the canvas.
    pub fn draw_rounded_rect(
        &mut self,
        brush: &Brush<Head<State>>,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) {
        self.draw_rounded_rect_at(
            self.canvas.size.into(),
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
        brush: &Brush<Head<State>>,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) {
        tracing::trace!("drawing rounded rect");

        self.push_command(CanvasCommand::Shape {
            paint_idx: brush.idx(),

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
    pub fn draw_path(&mut self, brush: &Brush<Head<State>>, path: Path) {
        self.draw_path_at(self.canvas.size.into(), brush, path);
    }

    /// Draws a path in the defined `rect`.
    pub fn draw_path_at(&mut self, rect: Rect, brush: &Brush<Head<State>>, path: Path) {
        tracing::trace!("drawing path");

        self.push_command(CanvasCommand::Shape {
            paint_idx: brush.idx(),

            rect,
            shape: Shape::Path(path),
        });
    }

    /// Draws text on the canvas. It will be wrapped to the `rect` of the canvas.
    pub fn draw_text<T>(&mut self, brush: &Brush<Head<State>>, text_style: TextStyle, text: T)
    where
        T: Into<Cow<'static, str>>,
    {
        self.draw_text_at(self.canvas.size.into(), brush, text_style, text);
    }

    /// Draws text on the canvas, ensuring it remains within the `rect`.
    pub fn draw_text_at<T>(
        &mut self,
        rect: Rect,
        brush: &Brush<Head<State>>,
        text_style: TextStyle,
        text: T,
    ) where
        T: Into<Cow<'static, str>>,
    {
        tracing::trace!("drawing text");

        self.push_command(CanvasCommand::Text {
            paint_idx: brush.idx(),

            rect,

            text_style,
            text: text.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn canvas_style() {}
}

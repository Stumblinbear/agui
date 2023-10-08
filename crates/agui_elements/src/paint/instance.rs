use std::rc::Rc;

use agui_core::{
    element::{
        render::ElementRender, widget::ElementWidget, ElementIntrinsicSizeContext, ElementUpdate,
    },
    render::canvas::{
        painter::{CanvasPainter, Head},
        Canvas,
    },
    unit::{IntrinsicDimension, Size},
    widget::Widget,
};

use super::WidgetPaint;

pub struct PaintElement<W>
where
    W: WidgetPaint,
{
    widget: Rc<W>,
}

impl<W> PaintElement<W>
where
    W: WidgetPaint,
{
    pub fn new(widget: Rc<W>) -> Self {
        Self { widget }
    }
}

impl<W> ElementWidget for PaintElement<W>
where
    W: WidgetPaint,
{
    fn widget_name(&self) -> &'static str {
        self.widget.widget_name()
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<W>() {
            self.widget = new_widget;

            ElementUpdate::RebuildNecessary
        } else {
            ElementUpdate::Invalid
        }
    }
}

impl<W> ElementRender for PaintElement<W>
where
    W: WidgetPaint,
{
    fn get_children(&self) -> Vec<Widget> {
        Vec::from_iter(self.widget.get_child())
    }

    fn intrinsic_size(
        &self,
        ctx: ElementIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        ctx.iter_children().next().map_or(0.0, |child| {
            child.compute_intrinsic_size(dimension, cross_extent)
        })
    }

    fn paint(&self, size: Size) -> Option<Canvas> {
        let mut canvas = Canvas {
            size,

            paints: Vec::default(),

            head: Vec::default(),
            children: Vec::default(),
            tail: None,
        };

        self.widget
            .paint(CanvasPainter::<Head<()>>::begin(&mut canvas));

        if !canvas.head.is_empty() || !canvas.children.is_empty() || canvas.tail.is_some() {
            Some(canvas)
        } else {
            None
        }
    }
}

impl<W> std::fmt::Debug for PaintElement<W>
where
    W: WidgetPaint + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("PaintElement");

        dbg.field("widget", &self.widget);

        dbg.finish()
    }
}

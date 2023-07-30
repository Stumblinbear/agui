use std::{any::Any, rc::Rc};

use crate::{
    callback::CallbackId,
    render::canvas::{
        painter::{CanvasPainter, Head},
        Canvas,
    },
    unit::Size,
    widget::{
        element::{ElementUpdate, WidgetBuildContext, WidgetCallbackContext, WidgetElement},
        AnyWidget, IntoChild, Widget, WidgetChild, WidgetPaint,
    },
};

pub struct PaintElement<W>
where
    W: AnyWidget + WidgetChild + WidgetPaint,
{
    widget: Rc<W>,
}

impl<W> PaintElement<W>
where
    W: AnyWidget + WidgetChild + WidgetPaint,
{
    pub fn new(widget: Rc<W>) -> Self {
        Self { widget }
    }
}

impl<W> WidgetElement for PaintElement<W>
where
    W: AnyWidget + WidgetChild + WidgetPaint,
{
    fn widget_name(&self) -> &'static str {
        self.widget.widget_name()
    }

    fn build(&mut self, _: WidgetBuildContext) -> Vec<Widget> {
        Vec::from_iter(self.widget.get_child().into_child())
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<W>() {
            self.widget = new_widget;

            ElementUpdate::RebuildNecessary
        } else {
            ElementUpdate::Invalid
        }
    }

    fn paint(&self, size: Size) -> Option<Canvas> {
        let mut canvas = Canvas {
            size,

            head: Vec::default(),
            children: Vec::default(),
            tail: None,
        };

        self.widget.paint(CanvasPainter::<Head>::begin(&mut canvas));

        if !canvas.head.is_empty() || !canvas.children.is_empty() || canvas.tail.is_some() {
            Some(canvas)
        } else {
            None
        }
    }

    fn call(&mut self, _: WidgetCallbackContext, _: CallbackId, _: Box<dyn Any>) -> bool {
        unreachable!("paint widgets do not have callbacks")
    }
}

impl<W> std::fmt::Debug for PaintElement<W>
where
    W: AnyWidget + WidgetChild + WidgetPaint + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("PaintElement");

        dbg.field("widget", &self.widget);

        dbg.finish()
    }
}

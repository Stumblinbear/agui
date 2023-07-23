use std::rc::Rc;

use crate::{
    callback::CallbackId,
    render::canvas::{
        painter::{CanvasPainter, Head},
        Canvas,
    },
    unit::{Data, Size},
    widget::{
        element::{ElementUpdate, WidgetBuildContext, WidgetCallbackContext, WidgetElement},
        AnyWidget, IntoChildren, WidgetChild, WidgetPaint, WidgetRef,
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
        let type_name = self.widget.widget_name();

        type_name
            .split('<')
            .next()
            .unwrap_or(type_name)
            .split("::")
            .last()
            .unwrap_or(type_name)
    }

    fn get_widget(&self) -> Rc<dyn AnyWidget> {
        Rc::clone(&self.widget) as Rc<dyn AnyWidget>
    }

    fn build(&mut self, _: WidgetBuildContext) -> Vec<WidgetRef> {
        self.widget.get_child().into_children()
    }

    fn update(&mut self, new_widget: &WidgetRef) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<W>() {
            if Rc::ptr_eq(&self.widget, &new_widget) {
                ElementUpdate::Noop
            } else {
                self.widget = new_widget;

                ElementUpdate::RebuildNecessary
            }
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

    fn call(&mut self, _: WidgetCallbackContext, _: CallbackId, _: &Box<dyn Data>) -> bool {
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

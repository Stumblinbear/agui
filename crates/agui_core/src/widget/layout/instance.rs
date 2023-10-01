use std::rc::Rc;

use crate::{
    render::element::ElementRender,
    unit::{Constraints, IntrinsicDimension, Size},
    widget::{
        element::{ElementUpdate, ElementWidget, WidgetIntrinsicSizeContext, WidgetLayoutContext},
        AnyWidget, IntrinsicSizeContext, LayoutContext, Widget, WidgetLayout,
    },
};

pub struct LayoutElement<W>
where
    W: AnyWidget + WidgetLayout,
{
    widget: Rc<W>,
}

impl<W> LayoutElement<W>
where
    W: AnyWidget + WidgetLayout,
{
    pub fn new(widget: Rc<W>) -> Self {
        Self { widget }
    }
}

impl<W> ElementWidget for LayoutElement<W>
where
    W: AnyWidget + WidgetLayout,
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

impl<W> ElementRender for LayoutElement<W>
where
    W: AnyWidget + WidgetLayout,
{
    fn get_children(&self) -> Vec<Widget> {
        self.widget.get_children()
    }

    fn intrinsic_size(
        &self,
        ctx: WidgetIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        self.widget.intrinsic_size(
            &mut IntrinsicSizeContext { widget_ctx: ctx },
            dimension,
            cross_extent,
        )
    }

    fn layout(&self, ctx: WidgetLayoutContext, constraints: Constraints) -> Size {
        self.widget
            .layout(&mut LayoutContext { widget_ctx: ctx }, constraints)
    }
}

impl<W> std::fmt::Debug for LayoutElement<W>
where
    W: AnyWidget + WidgetLayout + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("LayoutElement");

        dbg.field("widget", &self.widget);

        dbg.finish()
    }
}

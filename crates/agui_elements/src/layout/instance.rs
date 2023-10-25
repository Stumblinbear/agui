use std::rc::Rc;

use agui_core::{
    element::{
        render::ElementRender, widget::ElementWidget, ElementUpdate, RenderObjectHitTestContext,
        RenderObjectIntrinsicSizeContext, RenderObjectLayoutContext,
    },
    unit::{Constraints, HitTest, IntrinsicDimension, Offset, Size},
    widget::{AnyWidget, Widget},
};

use super::{HitTestContext, IntrinsicSizeContext, LayoutContext};

use super::WidgetLayout;

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
    fn children(&self) -> Vec<Widget> {
        self.widget.children()
    }

    fn intrinsic_size(
        &self,
        ctx: RenderObjectIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        self.widget.intrinsic_size(
            &mut IntrinsicSizeContext { inner: ctx },
            dimension,
            cross_extent,
        )
    }

    fn layout(&mut self, ctx: RenderObjectLayoutContext, constraints: Constraints) -> Size {
        self.widget
            .layout(&mut LayoutContext { inner: ctx }, constraints)
    }

    fn hit_test<'ctx>(
        &self,
        ctx: &'ctx mut RenderObjectHitTestContext<'ctx>,
        position: Offset,
    ) -> HitTest {
        self.widget
            .hit_test(&mut HitTestContext { inner: ctx }, position)
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

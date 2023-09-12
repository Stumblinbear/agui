use std::rc::Rc;

use crate::widget::{
    element::{ElementUpdate, WidgetBuildContext, WidgetElement, WidgetMountContext},
    AnyWidget, Widget, WidgetChild,
};

use super::RenderContextBoundary;

pub struct RenderContextBoundaryElement {
    widget: Rc<RenderContextBoundary>,
}

impl RenderContextBoundaryElement {
    pub fn new(widget: Rc<RenderContextBoundary>) -> Self {
        Self { widget }
    }
}

impl WidgetElement for RenderContextBoundaryElement {
    fn widget_name(&self) -> &'static str {
        self.widget.widget_name()
    }

    fn mount(&mut self, ctx: WidgetMountContext) {
        ctx.render_context_manager
            .create_render_context(ctx.element_id);
    }

    fn build(&mut self, _: WidgetBuildContext) -> Vec<Widget> {
        vec![self.widget.get_child()]
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<RenderContextBoundary>() {
            self.widget = new_widget;

            ElementUpdate::RebuildNecessary
        } else {
            ElementUpdate::Invalid
        }
    }
}

impl std::fmt::Debug for RenderContextBoundaryElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("RenderContextBoundaryElement");

        dbg.finish()
    }
}

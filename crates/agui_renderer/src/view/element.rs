use std::rc::Rc;

use agui_core::{
    element::{proxy::ElementProxy, widget::ElementWidget, ElementMountContext, ElementUpdate},
    widget::{AnyWidget, IntoWidget, Widget},
};

use crate::{CurrentRenderView, RenderView, RenderViewId, RenderViewManager};

pub struct RenderViewElement {
    widget: Rc<RenderView>,

    child: Widget,
}

impl RenderViewElement {
    pub fn new(widget: Rc<RenderView>) -> Self {
        let child = CurrentRenderView {
            id: RenderViewId::default(),

            child: widget.child.clone(),
        }
        .into_widget();

        Self { widget, child }
    }
}

impl ElementWidget for RenderViewElement {
    fn widget_name(&self) -> &'static str {
        self.widget.widget_name()
    }

    fn mount(&mut self, ctx: ElementMountContext) {
        if let Some(render_view_manager) = RenderViewManager::of_mut(ctx.plugins) {
            self.child = CurrentRenderView {
                id: render_view_manager.create_render_view(*ctx.element_id),

                child: self.widget.child.clone(),
            }
            .into_widget();
        }
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<RenderView>() {
            self.widget = new_widget;

            ElementUpdate::RebuildNecessary
        } else {
            ElementUpdate::Invalid
        }
    }
}

impl ElementProxy for RenderViewElement {
    fn child(&self) -> Widget {
        self.child.clone()
    }
}

impl std::fmt::Debug for RenderViewElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("RenderViewElement");

        dbg.finish()
    }
}

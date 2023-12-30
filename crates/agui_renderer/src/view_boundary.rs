use std::rc::Rc;

use agui_core::{
    element::{
        render::ElementRender, widget::ElementWidget, ElementBuilder, ElementMountContext,
        ElementType, ElementUpdate, RenderObjectBuildContext, RenderObjectUpdateContext,
    },
    render::{binding::ViewBinding, RenderObject, RenderObjectImpl},
    widget::{IntoWidget, Widget},
};
use agui_macros::WidgetProps;

use crate::{CurrentRenderView, RenderViewId, RenderViewManager};

#[derive(WidgetProps)]
pub struct ViewBoundary {
    pub binding: Rc<dyn ViewBinding>,

    pub child: Widget,
}

impl IntoWidget for ViewBoundary {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for ViewBoundary {
    fn create_element(self: Rc<Self>) -> ElementType
    where
        Self: Sized,
    {
        ElementType::Render(Box::new(ViewBoundaryElement::new(self)))
    }
}

pub struct ViewBoundaryElement {
    widget: Rc<ViewBoundary>,

    child: Widget,
}

impl ViewBoundaryElement {
    pub fn new(widget: Rc<ViewBoundary>) -> Self {
        let child = CurrentRenderView {
            id: RenderViewId::default(),

            child: widget.child.clone(),
        }
        .into_widget();

        Self { widget, child }
    }
}

impl ElementWidget for ViewBoundaryElement {
    fn mount(&mut self, ctx: &mut ElementMountContext) {
        if let Some(render_view_manager) = RenderViewManager::of_mut(ctx.plugins) {
            self.child = CurrentRenderView {
                id: render_view_manager.create_render_view(*ctx.element_id),

                child: self.widget.child.clone(),
            }
            .into_widget();
        }
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<ViewBoundary>() {
            self.widget = new_widget;

            ElementUpdate::RebuildNecessary
        } else {
            ElementUpdate::Invalid
        }
    }
}

impl ElementRender for ViewBoundaryElement {
    fn children(&self) -> Vec<Widget> {
        vec![self.child.clone()]
    }

    fn create_render_object(&mut self, ctx: &mut RenderObjectBuildContext) -> RenderObject {
        RenderObject::new(RenderViewBoundary)
    }

    fn is_valid_render_object(&self, render_object: &RenderObject) -> bool {
        render_object.is::<RenderViewBoundary>()
    }

    fn update_render_object(
        &mut self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut RenderObject,
    ) {
    }
}

impl std::fmt::Debug for ViewBoundaryElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("ViewBoundaryElement");

        dbg.finish()
    }
}

struct RenderViewBoundary;

impl RenderObjectImpl for RenderViewBoundary {
    fn is_sized_by_parent(&self) -> bool {
        true
    }
}

use std::rc::Rc;

use agui_core::{
    element::{
        render::ElementRender, widget::ElementWidget, ElementMountContext, ElementUpdate,
        RenderObjectBuildContext, RenderObjectUpdateContext,
    },
    render::{RenderObject, RenderObjectImpl},
    widget::{IntoWidget, Widget},
};

use crate::{CurrentRenderView, RenderViewId, RenderViewManager, View};

pub struct ViewElement {
    widget: Rc<View>,

    child: Widget,
}

impl ViewElement {
    pub fn new(widget: Rc<View>) -> Self {
        let child = CurrentRenderView {
            id: RenderViewId::default(),

            child: widget.child.clone(),
        }
        .into_widget();

        Self { widget, child }
    }
}

impl ElementWidget for ViewElement {
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
        if let Some(new_widget) = new_widget.downcast::<View>() {
            self.widget = new_widget;

            ElementUpdate::RebuildNecessary
        } else {
            ElementUpdate::Invalid
        }
    }
}

impl ElementRender for ViewElement {
    fn children(&self) -> Vec<Widget> {
        vec![self.child.clone()]
    }

    fn create_render_object(&mut self, ctx: &mut RenderObjectBuildContext) -> RenderObject {
        RenderObject::new(RenderView)
    }

    fn is_valid_render_object(&self, render_object: &RenderObject) -> bool {
        render_object.is::<RenderView>()
    }

    fn update_render_object(
        &mut self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut RenderObject,
    ) {
    }
}

impl std::fmt::Debug for ViewElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("ViewElement");

        dbg.finish()
    }
}

struct RenderView;

impl RenderObjectImpl for RenderView {
    fn is_sized_by_parent(&self) -> bool {
        true
    }
}

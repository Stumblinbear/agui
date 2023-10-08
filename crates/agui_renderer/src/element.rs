use std::rc::Rc;

use agui_core::{
    element::{
        proxy::ElementProxy, widget::ElementWidget, ContextElement, ElementMountContext,
        ElementUpdate,
    },
    plugin::context::ContextPluginsMut,
    widget::{AnyWidget, Widget},
};

use crate::{RenderView, RenderViewPlugin};

pub struct RenderViewElement {
    widget: Rc<RenderView>,
}

impl RenderViewElement {
    pub fn new(widget: Rc<RenderView>) -> Self {
        Self { widget }
    }
}

impl ElementWidget for RenderViewElement {
    fn widget_name(&self) -> &'static str {
        self.widget.widget_name()
    }

    #[allow(unused_variables)]
    fn mount(&mut self, mut ctx: ElementMountContext) {
        let element_id = ctx.get_element_id();

        if let Some(render_view_plugin) = ctx.get_plugins_mut().get_mut::<RenderViewPlugin>() {
            render_view_plugin.create_render_view(element_id);
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
    fn get_child(&self) -> Widget {
        self.widget.child.clone()
    }
}

impl std::fmt::Debug for RenderViewElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("RenderViewElement");

        dbg.finish()
    }
}

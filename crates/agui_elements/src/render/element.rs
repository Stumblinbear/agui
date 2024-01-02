use std::rc::Rc;

use agui_core::{
    element::{
        render::ElementRender, widget::ElementWidget, ElementUpdate, RenderObjectUpdateContext,
    },
    render::RenderObject,
    widget::Widget,
};

use super::{RenderObjectCreateContext, RenderObjectWidget};

pub struct RenderObjectWidgetElement<W>
where
    W: RenderObjectWidget,
{
    widget: Rc<W>,
}

impl<W> RenderObjectWidgetElement<W>
where
    W: RenderObjectWidget,
{
    pub fn new(widget: Rc<W>) -> Self {
        Self { widget }
    }
}

impl<W> ElementWidget for RenderObjectWidgetElement<W>
where
    W: RenderObjectWidget,
{
    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<W>() {
            self.widget = new_widget;

            ElementUpdate::RebuildNecessary
        } else {
            ElementUpdate::Invalid
        }
    }
}

impl<W> ElementRender for RenderObjectWidgetElement<W>
where
    W: RenderObjectWidget,
{
    fn children(&self) -> Vec<Widget> {
        self.widget.children()
    }

    fn create_render_object(&mut self, ctx: &mut RenderObjectCreateContext) -> RenderObject {
        RenderObject::new(self.widget.create_render_object(ctx))
    }

    fn is_valid_render_object(&self, render_object: &RenderObject) -> bool {
        render_object.is::<W::RenderObject>()
    }

    fn update_render_object(
        &mut self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut RenderObject,
    ) {
        self.widget.update_render_object(
            ctx,
            render_object
                .downcast_mut::<W::RenderObject>()
                .expect("render object type mismatch"),
        );
    }
}

impl<W> std::fmt::Debug for RenderObjectWidgetElement<W>
where
    W: RenderObjectWidget + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("RenderObjectWidgetElement");
        dbg.field("widget", &self.widget);
        dbg.finish()
    }
}

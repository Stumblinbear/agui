use std::rc::Rc;

use agui_core::{
    element::{
        lifecycle::ElementLifecycle, render::ElementRender, widget::ElementWidget,
        ElementComparison, RenderObjectUpdateContext,
    },
    render::object::RenderObject,
    util::ptr_eq::PtrEqual,
    widget::{AnyWidget, Widget},
};

use super::{RenderObjectCreateContext, RenderObjectWidget};

pub struct RenderObjectWidgetElement<W> {
    widget: Rc<W>,
}

impl<W> RenderObjectWidgetElement<W> {
    pub fn new(widget: Rc<W>) -> Self {
        Self { widget }
    }
}

impl<W> ElementLifecycle for RenderObjectWidgetElement<W>
where
    W: AnyWidget + RenderObjectWidget,
{
    fn update(&mut self, new_widget: &Widget) -> ElementComparison {
        if new_widget.is_exact_ptr(&self.widget) {
            return ElementComparison::Identical;
        }

        if let Some(new_widget) = new_widget.downcast::<W>() {
            self.widget = new_widget;

            ElementComparison::Changed
        } else {
            ElementComparison::Invalid
        }
    }
}

impl<W> ElementWidget for RenderObjectWidgetElement<W>
where
    W: AnyWidget + RenderObjectWidget,
{
    type Widget = W;

    fn widget(&self) -> &Rc<Self::Widget> {
        &self.widget
    }
}

impl<W> ElementRender for RenderObjectWidgetElement<W>
where
    W: AnyWidget + RenderObjectWidget,
{
    fn children(&self) -> Vec<Widget> {
        self.widget.children()
    }

    fn create_render_object(&self, ctx: &mut RenderObjectCreateContext) -> RenderObject {
        RenderObject::new(self.widget.create_render_object(ctx))
    }

    fn is_valid_render_object(&self, render_object: &RenderObject) -> bool {
        render_object.is::<W::RenderObject>()
    }

    fn update_render_object(
        &self,
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

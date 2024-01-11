use agui_core::{
    element::{
        render::ElementRender, view::ElementView, widget::ElementWidget, ElementBuilder,
        ElementType, ElementUpdate, RenderObjectCreateContext, RenderObjectUpdateContext,
    },
    render::{
        binding::ViewBinding,
        object::{RenderObject, RenderObjectImpl},
    },
    widget::Widget,
};
use agui_macros::WidgetProps;

use crate::renderer::binding::VelloViewBinding;

#[derive(WidgetProps)]
pub struct VelloView {
    pub binding: VelloViewBinding,

    #[prop(into)]
    pub child: Widget,
}

impl ElementBuilder for VelloView {
    fn create_element(self: std::rc::Rc<Self>) -> ElementType
    where
        Self: Sized,
    {
        ElementType::new_view(VelloViewElement::new(
            self.binding.clone(),
            self.child.clone(),
        ))
    }
}

struct VelloViewElement {
    binding: VelloViewBinding,

    child: Widget,
}

impl VelloViewElement {
    pub fn new(binding: VelloViewBinding, child: Widget) -> Self {
        Self { binding, child }
    }
}

impl ElementWidget for VelloViewElement {
    fn update(&mut self, _: &Widget) -> ElementUpdate {
        ElementUpdate::Invalid
    }
}

impl ElementRender for VelloViewElement {
    fn children(&self) -> Vec<Widget> {
        vec![self.child.clone()]
    }

    fn create_render_object(&mut self, _: &mut RenderObjectCreateContext) -> RenderObject {
        RenderObject::new(RenderVelloView)
    }

    fn is_valid_render_object(&self, render_object: &RenderObject) -> bool {
        render_object.is::<RenderVelloView>()
    }

    fn update_render_object(&mut self, _: &mut RenderObjectUpdateContext, _: &mut RenderObject) {}
}

impl ElementView for VelloViewElement {
    fn create_binding(&mut self) -> Box<dyn ViewBinding> {
        Box::new(self.binding.clone())
    }
}

struct RenderVelloView;

impl RenderObjectImpl for RenderVelloView {
    fn is_sized_by_parent(&self) -> bool {
        true
    }
}

use agui_core::{
    element::{
        lifecycle::ElementLifecycle, render::ElementRender, view::ElementView, ElementBuilder,
        ElementComparison, ElementType, RenderObjectCreateContext, RenderObjectUpdateContext,
    },
    render::{
        object::{RenderObject, RenderObjectImpl},
        view::View,
    },
    widget::{IntoWidget, Widget},
};
use agui_macros::WidgetProps;

use crate::view::VelloView;

#[derive(WidgetProps)]
pub struct VelloViewBinding {
    pub view: VelloView,

    #[prop(into)]
    pub child: Widget,
}

impl IntoWidget for VelloViewBinding {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for VelloViewBinding {
    type Element = VelloViewElement;

    fn create_element(self: std::rc::Rc<Self>) -> ElementType
    where
        Self: Sized,
    {
        ElementType::new_view(VelloViewElement::new(self.view.clone(), self.child.clone()))
    }
}

pub struct VelloViewElement {
    view: VelloView,

    child: Widget,
}

impl VelloViewElement {
    pub fn new(view: VelloView, child: Widget) -> Self {
        Self { view, child }
    }
}

impl ElementLifecycle for VelloViewElement {
    fn update(&mut self, _: &Widget) -> ElementComparison {
        ElementComparison::Invalid
    }
}

impl ElementRender for VelloViewElement {
    fn children(&self) -> Vec<Widget> {
        vec![self.child.clone()]
    }

    fn create_render_object(&self, _: &mut RenderObjectCreateContext) -> RenderObject {
        RenderObject::new(RenderVelloView)
    }

    fn is_valid_render_object(&self, render_object: &RenderObject) -> bool {
        render_object.is::<RenderVelloView>()
    }

    fn update_render_object(&self, _: &mut RenderObjectUpdateContext, _: &mut RenderObject) {}
}

impl ElementView for VelloViewElement {
    fn create_view(&self) -> Box<dyn View> {
        Box::new(self.view.clone())
    }
}

struct RenderVelloView;

impl RenderObjectImpl for RenderVelloView {
    fn is_sized_by_parent(&self) -> bool {
        true
    }
}

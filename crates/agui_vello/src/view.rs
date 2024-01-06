use std::rc::Rc;

use agui_core::{
    element::{
        render::ElementRender, view::ElementView, widget::ElementWidget, ElementBuilder,
        ElementType, ElementUpdate, RenderObjectCreateContext, RenderObjectUpdateContext,
    },
    render::{
        binding::ViewBinding,
        canvas::Canvas,
        object::{RenderObject, RenderObjectImpl},
        RenderObjectId,
    },
    unit::{Offset, Size},
    widget::Widget,
};
use agui_macros::WidgetProps;

#[derive(WidgetProps)]
pub struct VelloView<F>
where
    F: Fn() -> VelloViewBinding + 'static,
{
    pub binding: F,

    #[prop(into)]
    pub child: Widget,
}

impl<F> ElementBuilder for VelloView<F>
where
    F: Fn() -> VelloViewBinding + 'static,
{
    fn create_element(self: std::rc::Rc<Self>) -> ElementType
    where
        Self: Sized,
    {
        ElementType::new_view(VelloViewElement::new((self.binding)(), self.child.clone()))
    }
}

struct VelloViewElement {
    binding: Rc<dyn ViewBinding>,

    child: Widget,
}

impl VelloViewElement {
    pub fn new(binding: VelloViewBinding, child: Widget) -> Self {
        Self {
            binding: Rc::new(binding),

            child,
        }
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
    fn binding(&self) -> &Rc<dyn ViewBinding> {
        &self.binding
    }
}

struct RenderVelloView;

impl RenderObjectImpl for RenderVelloView {
    fn is_sized_by_parent(&self) -> bool {
        true
    }
}

pub struct VelloViewBinding;

impl ViewBinding for VelloViewBinding {
    fn on_attach(
        &self,
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
    ) {
        println!(
            "VelloViewElement::on_attach {:?} {:?}",
            parent_render_object_id, render_object_id
        );
    }

    fn on_detach(&self, render_object_id: RenderObjectId) {
        println!("VelloViewElement::on_detach {:?}", render_object_id);
    }

    fn on_size_changed(&self, render_object_id: RenderObjectId, size: Size) {
        println!(
            "VelloViewElement::on_size_changed {:?} {:?}",
            render_object_id, size
        );
    }

    fn on_offset_changed(&self, render_object_id: RenderObjectId, offset: Offset) {
        println!(
            "VelloViewElement::on_offset_changed {:?} {:?}",
            render_object_id, offset
        );
    }

    fn on_paint(&self, render_object_id: RenderObjectId, canvas: Canvas) {
        println!(
            "VelloViewElement::on_paint {:?} {:?}",
            render_object_id, canvas
        );
    }
}

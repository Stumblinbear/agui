use std::rc::Rc;

use agui_core::{
    element::{
        view::ElementView, widget::ElementWidget, ElementBuilder, ElementType, ElementUpdate,
    },
    render::RenderObjectId,
    widget::{AnyWidget, IntoWidget, Widget},
};
use agui_macros::WidgetProps;

#[derive(WidgetProps)]
pub struct VelloView {
    #[prop(into)]
    pub child: Widget,
}

impl IntoWidget for VelloView {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for VelloView {
    fn create_element(self: std::rc::Rc<Self>) -> ElementType
    where
        Self: Sized,
    {
        ElementType::View(Box::new(VelloViewElement::new(self)))
    }
}

struct VelloViewElement {
    widget: Rc<VelloView>,
}

impl VelloViewElement {
    pub fn new(widget: Rc<VelloView>) -> Self {
        Self { widget }
    }
}

impl ElementWidget for VelloViewElement {
    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<VelloView>() {
            if self.widget.child == new_widget.child {
                return ElementUpdate::Noop;
            }

            self.widget = new_widget;

            ElementUpdate::RebuildNecessary
        } else {
            ElementUpdate::Invalid
        }
    }
}

impl ElementView for VelloViewElement {
    fn child(&self) -> Widget {
        self.widget.child.clone()
    }

    fn on_attach(
        &mut self,
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
    ) {
        println!(
            "VelloViewElement::on_attach {:?} {:?}",
            parent_render_object_id, render_object_id
        );
    }

    fn on_detach(&mut self, render_object_id: RenderObjectId) {
        println!("VelloViewElement::on_detach {:?}", render_object_id);
    }

    fn on_needs_visual_update(&mut self, render_object_id: RenderObjectId) {
        println!(
            "VelloViewElement::on_needs_visual_update {:?}",
            render_object_id
        );
    }

    fn on_needs_semantics_update(&mut self, render_object_id: RenderObjectId) {
        println!(
            "VelloViewElement::on_needs_semantics_update {:?}",
            render_object_id
        );
    }

    fn redraw(&mut self) {
        println!("VelloViewElement::redraw");
    }

    fn render(&self) {
        println!("VelloViewElement::render");
    }
}

impl std::fmt::Debug for VelloViewElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VelloViewElement").finish()
    }
}

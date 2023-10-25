use crate::{render::RenderObject, widget::Widget};

use super::widget::ElementWidget;

pub trait ElementRender: ElementWidget {
    fn children(&self) -> Vec<Widget>;

    fn create_render_object(&self) -> RenderObject;

    fn update_render_object(&self, render_object: &mut RenderObject);
}

impl std::fmt::Debug for Box<dyn ElementRender> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(self.widget_name()).finish_non_exhaustive()
    }
}

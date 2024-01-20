use agui_core::{
    element::{RenderObjectCreateContext, RenderObjectUpdateContext},
    render::object::RenderObjectImpl,
    widget::Widget,
};

mod element;

pub use element::*;

pub trait RenderObjectWidget: Sized {
    type RenderObject: RenderObjectImpl;

    fn children(&self) -> Vec<Widget>;

    fn create_render_object(&self, ctx: &mut RenderObjectCreateContext) -> Self::RenderObject;

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    );
}

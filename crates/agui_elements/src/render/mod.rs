use agui_core::{
    element::{RenderObjectBuildContext, RenderObjectUpdateContext},
    render::RenderObjectImpl,
    widget::{AnyWidget, Widget},
};

mod element;

pub use element::*;

pub trait RenderObjectWidget: AnyWidget {
    type RenderObject: RenderObjectImpl;

    fn children(&self) -> Vec<Widget>;

    fn create_render_object(&self, ctx: &mut RenderObjectBuildContext) -> Self::RenderObject;

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    );
}

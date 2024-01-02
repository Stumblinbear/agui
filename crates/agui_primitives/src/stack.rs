use agui_core::{
    element::{RenderObjectCreateContext, RenderObjectUpdateContext},
    render::object::{
        RenderObjectImpl, RenderObjectIntrinsicSizeContext, RenderObjectLayoutContext,
    },
    unit::{Constraints, IntrinsicDimension, Size},
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;

#[derive(RenderObjectWidget, Debug)]
pub struct Stack {
    #[prop(into)]
    pub children: Vec<Widget>,
}

impl RenderObjectWidget for Stack {
    type RenderObject = RenderStack;

    fn children(&self) -> Vec<Widget> {
        self.children.clone()
    }

    fn create_render_object(&self, _: &mut RenderObjectCreateContext) -> Self::RenderObject {
        RenderStack
    }

    fn update_render_object(&self, _: &mut RenderObjectUpdateContext, _: &mut Self::RenderObject) {}
}

#[derive(Debug)]
pub struct RenderStack;

impl RenderObjectImpl for RenderStack {
    // TODO: make this actually work properly
    fn intrinsic_size(
        &self,
        _: &mut RenderObjectIntrinsicSizeContext,
        _: IntrinsicDimension,
        _: f32,
    ) -> f32 {
        0.0
    }

    // TODO: make this actually work properly
    fn layout(&self, ctx: &mut RenderObjectLayoutContext, constraints: Constraints) -> Size {
        let mut children = ctx.iter_children_mut();

        let mut size = constraints.biggest();

        while let Some(mut child) = children.next() {
            size = child.compute_layout(constraints);
        }

        size
    }
}

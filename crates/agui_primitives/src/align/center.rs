use agui_core::{
    element::{RenderObjectCreateContext, RenderObjectUpdateContext},
    unit::Alignment,
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;

use super::aligned_box::RenderAlignedBox;

#[derive(RenderObjectWidget, Debug)]
#[props(default)]
pub struct Center {
    pub width_factor: Option<f32>,
    pub height_factor: Option<f32>,

    #[prop(into)]
    pub child: Option<Widget>,
}

impl RenderObjectWidget for Center {
    type RenderObject = RenderAlignedBox;

    fn children(&self) -> Vec<Widget> {
        Vec::from_iter(self.child.clone())
    }

    fn create_render_object(&self, _: &mut RenderObjectCreateContext) -> Self::RenderObject {
        RenderAlignedBox {
            alignment: Alignment::CENTER,

            width_factor: self.width_factor,
            height_factor: self.height_factor,
        }
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    ) {
        render_object.update_alignment(ctx, Alignment::CENTER);

        render_object.update_width_factor(ctx, self.width_factor);
        render_object.update_height_factor(ctx, self.height_factor);
    }
}

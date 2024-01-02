use agui_core::{
    element::{RenderObjectCreateContext, RenderObjectUpdateContext},
    unit::{Axis, ClipBehavior, TextDirection},
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;

use crate::flex::{CrossAxisAlignment, MainAxisAlignment, MainAxisSize, VerticalDirection};

use super::{FlexChildParams, RenderFlex};

#[derive(RenderObjectWidget, Debug)]
#[props(default)]
pub struct Row {
    pub main_axis_size: MainAxisSize,

    pub main_axis_alignment: MainAxisAlignment,
    pub cross_axis_alignment: CrossAxisAlignment,
    pub vertical_direction: VerticalDirection,

    pub text_direction: Option<TextDirection>,

    pub clip_behavior: ClipBehavior,

    pub children: Vec<Widget>,
}

impl RenderObjectWidget for Row {
    type RenderObject = RenderFlex;

    fn children(&self) -> Vec<Widget> {
        self.children.clone()
    }

    fn create_render_object(&self, _: &mut RenderObjectCreateContext) -> Self::RenderObject {
        RenderFlex {
            direction: Axis::Horizontal,

            main_axis_size: self.main_axis_size,
            main_axis_alignment: self.main_axis_alignment,
            cross_axis_alignment: self.cross_axis_alignment,
            vertical_direction: self.vertical_direction,

            text_direction: self.text_direction,

            clip_behavior: self.clip_behavior,

            children_params: self.children.iter().map(FlexChildParams::from).collect(),
        }
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    ) {
        render_object.update_direction(ctx, Axis::Vertical);

        render_object.update_main_axis_size(ctx, self.main_axis_size);
        render_object.update_main_axis_alignment(ctx, self.main_axis_alignment);
        render_object.update_cross_axis_alignment(ctx, self.cross_axis_alignment);
        render_object.update_vertical_direction(ctx, self.vertical_direction);

        render_object.update_text_direction(ctx, self.text_direction);

        render_object.update_clip_behavior(ctx, self.clip_behavior);

        render_object.update_children_params(
            ctx,
            self.children.iter().map(FlexChildParams::from).collect(),
        );
    }
}

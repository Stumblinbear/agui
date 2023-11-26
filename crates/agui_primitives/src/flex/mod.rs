use agui_core::{
    element::{RenderObjectBuildContext, RenderObjectUpdateContext},
    unit::{Axis, ClipBehavior, TextDirection},
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;

mod column;
mod flex_render_object;
mod flexible;
mod params;
mod row;

pub use column::*;
pub use flex_render_object::*;
pub use flexible::*;
pub use params::*;
pub use row::*;

#[derive(RenderObjectWidget, Debug)]
#[props(default)]
pub struct Flex {
    #[prop(!default)]
    pub direction: Axis,

    pub main_axis_size: MainAxisSize,
    pub main_axis_alignment: MainAxisAlignment,
    pub cross_axis_alignment: CrossAxisAlignment,
    pub vertical_direction: VerticalDirection,

    pub text_direction: Option<TextDirection>,

    pub clip_behavior: ClipBehavior,

    // #[prop(into, transform = |widgets: impl IntoIterator<Item = Widget>| widgets.into_iter().map(FlexChild::from).collect())]
    pub children: Vec<Widget>,
}

impl RenderObjectWidget for Flex {
    type RenderObject = RenderFlex;

    fn children(&self) -> Vec<Widget> {
        self.children.clone()
    }

    fn create_render_object(&self, _: &mut RenderObjectBuildContext) -> Self::RenderObject {
        RenderFlex {
            direction: self.direction,

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
        render_object.update_direction(ctx, self.direction);

        render_object.update_main_axis_size(ctx, self.main_axis_size);
        render_object.update_main_axis_alignment(ctx, self.main_axis_alignment);
        render_object.update_cross_axis_alignment(ctx, self.cross_axis_alignment);
        render_object.update_vertical_direction(ctx, self.vertical_direction);

        render_object.update_text_direction(ctx, self.text_direction);

        render_object.update_clip_behavior(ctx, self.clip_behavior);
    }
}

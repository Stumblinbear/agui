use agui_core::{
    unit::{Axis, ClipBehavior, TextDirection},
    widget::{BuildContext, Children, WidgetView},
};
use agui_macros::StatelessWidget;

use crate::{
    CrossAxisAlignment, Flex, Flexible, MainAxisAlignment, MainAxisSize, VerticalDirection,
};

#[derive(StatelessWidget, Debug, Default)]
pub struct Row {
    pub main_axis_size: MainAxisSize,

    pub main_axis_alignment: MainAxisAlignment,
    pub cross_axis_alignment: CrossAxisAlignment,
    pub vertical_direction: VerticalDirection,

    pub text_direction: Option<TextDirection>,

    pub clip_behavior: ClipBehavior,

    pub children: Vec<Flexible>,
}

impl WidgetView for Row {
    fn build(&self, _: &mut BuildContext<Self>) -> Children {
        Children::from([Flex {
            direction: Axis::Vertical,

            main_axis_size: self.main_axis_size,

            main_axis_alignment: self.main_axis_alignment,
            cross_axis_alignment: self.cross_axis_alignment,
            vertical_direction: self.vertical_direction,

            text_direction: self.text_direction,

            clip_behavior: self.clip_behavior,

            children: self.children.clone(),
        }])
    }
}

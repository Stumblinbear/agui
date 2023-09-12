use agui_core::{
    unit::{Axis, ClipBehavior, TextDirection},
    widget::{BuildContext, IntoWidget, Widget, WidgetBuild},
};
use agui_macros::StatelessWidget;

use crate::flex::{
    CrossAxisAlignment, Flex, Flexible, MainAxisAlignment, MainAxisSize, VerticalDirection,
};

#[derive(Debug, StatelessWidget)]
#[prop(field_defaults(default))]
pub struct Row {
    pub main_axis_size: MainAxisSize,

    pub main_axis_alignment: MainAxisAlignment,
    pub cross_axis_alignment: CrossAxisAlignment,
    pub vertical_direction: VerticalDirection,

    pub text_direction: Option<TextDirection>,

    pub clip_behavior: ClipBehavior,

    #[prop(setter(into))]
    pub children: Vec<Flexible>,
}

impl WidgetBuild for Row {
    fn build(&self, _: &mut BuildContext<Self>) -> Widget {
        Flex {
            direction: Axis::Vertical,

            main_axis_size: self.main_axis_size,

            main_axis_alignment: self.main_axis_alignment,
            cross_axis_alignment: self.cross_axis_alignment,
            vertical_direction: self.vertical_direction,

            text_direction: self.text_direction,

            clip_behavior: self.clip_behavior,

            children: self.children.clone(),
        }
        .into_widget()
    }
}

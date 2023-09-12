use agui_core::{
    unit::{Axis, ClipBehavior, TextDirection},
    widget::{IntoWidget, Widget},
};

use crate::flex::{
    CrossAxisAlignment, Flex, Flexible, MainAxisAlignment, MainAxisSize, VerticalDirection,
};

#[derive(Debug, Default)]
pub struct Column {
    pub main_axis_size: MainAxisSize,

    pub main_axis_alignment: MainAxisAlignment,
    pub cross_axis_alignment: CrossAxisAlignment,
    pub vertical_direction: VerticalDirection,

    pub text_direction: Option<TextDirection>,

    pub clip_behavior: ClipBehavior,

    pub children: Vec<Flexible>,
}

impl Column {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn with_main_axis_size(mut self, main_axis_size: MainAxisSize) -> Self {
        self.main_axis_size = main_axis_size;

        self
    }

    pub const fn with_main_axis_alignment(
        mut self,
        main_axis_alignment: MainAxisAlignment,
    ) -> Self {
        self.main_axis_alignment = main_axis_alignment;

        self
    }

    pub const fn with_cross_axis_alignment(
        mut self,
        cross_axis_alignment: CrossAxisAlignment,
    ) -> Self {
        self.cross_axis_alignment = cross_axis_alignment;

        self
    }

    pub const fn with_vertical_direction(mut self, vertical_direction: VerticalDirection) -> Self {
        self.vertical_direction = vertical_direction;

        self
    }

    pub const fn with_text_direction(mut self, text_direction: TextDirection) -> Self {
        self.text_direction = Some(text_direction);

        self
    }

    pub const fn with_clip_behavior(mut self, clip_behavior: ClipBehavior) -> Self {
        self.clip_behavior = clip_behavior;

        self
    }

    pub fn with_children(
        mut self,
        children: impl IntoIterator<Item = impl Into<Flexible>>,
    ) -> Self {
        self.children = children.into_iter().map(Into::into).collect();

        self
    }
}

impl IntoWidget for Column {
    fn into_widget(self) -> Widget {
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

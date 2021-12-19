use std::any::TypeId;

use crate::{
    widget::{BuildResult, Layout, Size, Widget},
    WidgetContext,
};

#[derive(Default)]
pub struct Text {
    pub size: Size,
    pub text: String,
}

impl Widget for Text {
    fn get_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn layout(&self) -> Option<&Layout> {
        None
    }

    fn build(&self, _ctx: &WidgetContext) -> BuildResult {
        BuildResult::Empty
    }
}

impl From<Text> for Box<dyn Widget> {
    fn from(text: Text) -> Self {
        Box::new(text)
    }
}

impl From<Text> for Option<Box<dyn Widget>> {
    fn from(text: Text) -> Self {
        Some(Box::new(text))
    }
}

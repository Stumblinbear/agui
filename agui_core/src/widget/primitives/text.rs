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

impl Into<Box<dyn Widget>> for Text {
    fn into(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}

impl Into<Option<Box<dyn Widget>>> for Text {
    fn into(self) -> Option<Box<dyn Widget>> {
        Some(Box::new(self))
    }
}

use std::any::TypeId;

use crate::{
    widget::{BuildResult, Layout, Widget},
    WidgetContext,
};

#[derive(Default)]
pub struct Quad {
    pub layout: Layout,

    pub clip: bool,
    pub child: Option<Box<dyn Widget>>,
}

impl Widget for Quad {
    fn get_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn layout(&self) -> Option<&Layout> {
        Some(&self.layout)
    }

    fn build(&self, _ctx: &WidgetContext) -> BuildResult {
        BuildResult::Empty
    }
}

impl Into<Box<dyn Widget>> for Quad {
    fn into(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}

impl Into<Option<Box<dyn Widget>>> for Quad {
    fn into(self) -> Option<Box<dyn Widget>> {
        Some(Box::new(self))
    }
}

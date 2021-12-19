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

impl From<Quad> for Box<dyn Widget> {
    fn from(quad: Quad) -> Box<dyn Widget> {
        Box::new(quad)
    }
}

impl From<Quad> for Option<Box<dyn Widget>> {
    fn from(quad: Quad) -> Option<Box<dyn Widget>> {
        Some(Box::new(quad))
    }
}

use agui_core::{
    render::color::Color, unit::Layout, BuildResult, WidgetContext, WidgetImpl, WidgetRef,
};
use agui_macros::Widget;

#[derive(Default, Widget)]
pub struct Quad {
    pub layout: Layout,
    pub color: Color,

    pub clip: bool,
    pub child: Option<WidgetRef>,
}

impl WidgetImpl for Quad {
    fn layout(&self) -> Option<&Layout> {
        Some(&self.layout)
    }

    fn build(&self, _ctx: &WidgetContext) -> BuildResult {
        self.child
            .as_ref()
            .map_or(BuildResult::Empty, |child| BuildResult::One(child.clone()))
    }
}
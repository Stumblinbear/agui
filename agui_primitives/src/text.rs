use agui_core::{
    unit::{Layout, Sizing},
    BuildResult, WidgetContext, WidgetImpl,
};
use agui_macros::Widget;

#[derive(Default, Widget)]
pub struct Text {
    pub size: Sizing,
    pub text: String,
}

impl WidgetImpl for Text {
    fn layout(&self) -> Option<&Layout> {
        None
    }

    fn build(&self, _ctx: &WidgetContext) -> BuildResult {
        BuildResult::Empty
    }
}

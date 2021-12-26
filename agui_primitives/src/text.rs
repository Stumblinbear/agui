use agui_core::{
    context::WidgetContext,
    unit::Sizing,
    widget::{BuildResult, WidgetImpl},
};
use agui_macros::Widget;

#[derive(Default, Widget)]
pub struct Text {
    pub size: Sizing,
    pub text: String,
}

impl WidgetImpl for Text {
    fn build(&self, _ctx: &WidgetContext) -> BuildResult {
        BuildResult::Empty
    }
}

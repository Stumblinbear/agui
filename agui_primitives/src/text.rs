use agui_core::{widget::{BuildResult, WidgetImpl}, context::WidgetContext, unit::Sizing};
use agui_macros::Widget;

#[derive(Debug, Default, Widget)]
pub struct Text {
    pub size: Sizing,
    pub text: String,
}

impl WidgetImpl for Text {
    fn build(&self, _ctx: &WidgetContext) -> BuildResult {
        BuildResult::Empty
    }
}

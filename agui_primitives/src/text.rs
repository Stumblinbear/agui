use agui_core::{
    context::WidgetContext,
    unit::Sizing,
    widget::{BuildResult, WidgetBuilder},
};
use agui_macros::Widget;

#[derive(Default, Widget)]
pub struct Text {
    pub size: Sizing,
    pub text: String,
}

impl WidgetBuilder for Text {
    fn build(&self, _ctx: &WidgetContext) -> BuildResult {
        BuildResult::Empty
    }
}

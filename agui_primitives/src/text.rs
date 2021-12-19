use agui_core::{WidgetContext, unit::{Layout, Sizing}, WidgetImpl, BuildResult, WidgetRef};
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

impl From<Text> for WidgetRef {
    fn from(text: Text) -> Self {
        Self::new(text)
    }
}

impl From<Text> for Option<WidgetRef> {
    fn from(text: Text) -> Self {
        Some(WidgetRef::new(text))
    }
}

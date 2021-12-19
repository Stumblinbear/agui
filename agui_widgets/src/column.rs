use agui_core::{
    widget::{BuildResult, Layout, WidgetRef, WidgetImpl},
    WidgetContext,
};
use agui_macros::Widget;

#[derive(Default, Widget)]
#[widget(layout = "column")]
pub struct Column {
    pub layout: Layout,

    pub children: Vec<WidgetRef>,
}

impl WidgetImpl for Column {
    fn layout(&self) -> Option<&Layout> {
        Some(&self.layout)
    }

    fn build(&self, _ctx: &WidgetContext) -> BuildResult {
        BuildResult::Many(self.children.clone())
    }
}

use agui_core::{
    context::WidgetContext,
    layout::LayoutRef,
    widget::{BuildResult, WidgetImpl, WidgetRef},
};
use agui_macros::Widget;

#[derive(Default, Widget)]
#[widget(layout = "column")]
pub struct Column {
    pub layout: LayoutRef,

    pub children: Vec<WidgetRef>,
}

impl WidgetImpl for Column {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        ctx.set_layout(LayoutRef::clone(&self.layout));

        (&self.children).into()
    }
}

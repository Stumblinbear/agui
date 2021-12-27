use agui_core::{
    context::WidgetContext,
    layout::Layout,
    widget::{BuildResult, WidgetBuilder, WidgetRef},
    Ref,
};
use agui_macros::Widget;

#[derive(Default, Widget)]
#[widget(layout = "column")]
pub struct Column {
    pub layout: Ref<Layout>,

    pub children: Vec<WidgetRef>,
}

impl WidgetBuilder for Column {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        ctx.set_layout(Ref::clone(&self.layout));

        (&self.children).into()
    }
}

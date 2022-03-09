use agui_core::{
    unit::{Layout, LayoutType, Units},
    widget::{BuildContext, BuildResult, WidgetBuilder, WidgetRef},
};
use agui_macros::Widget;

#[derive(Default, Widget)]
pub struct Row {
    pub layout: Layout,

    pub spacing: Units,

    pub children: Vec<WidgetRef>,
}

impl WidgetBuilder for Row {
    fn build(&self, ctx: &mut BuildContext) -> BuildResult {
        ctx.set_layout_type(LayoutType::Row {
            spacing: self.spacing,
        });

        ctx.set_layout(Layout::clone(&self.layout));

        (&self.children).into()
    }
}

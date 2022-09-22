use agui_core::{
    unit::{Layout, LayoutType, Units},
    widget::{BuildContext, BuildResult, WidgetBuilder, WidgetRef},
};

#[derive(Debug, Default, PartialEq)]
pub struct Column {
    pub layout: Layout,

    pub spacing: Units,

    pub children: Vec<WidgetRef>,
}

impl WidgetBuilder for Column {
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        ctx.set_layout_type(LayoutType::Column {
            spacing: self.spacing,
        });

        ctx.set_layout(Layout::clone(&self.layout));

        (&self.children).into()
    }
}

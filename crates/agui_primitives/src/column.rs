use agui_core::{
    unit::{Layout, LayoutType, Units},
    widget::{BuildContext, BuildResult, StatelessWidget, Widget},
};

#[derive(Debug, Default)]
pub struct Column {
    pub layout: Layout,

    pub spacing: Units,

    pub children: Vec<Widget>,
}

impl StatelessWidget for Column {
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        ctx.set_layout_type(LayoutType::Column {
            spacing: self.spacing,
        });

        ctx.set_layout(Layout::clone(&self.layout));

        (&self.children).into()
    }
}

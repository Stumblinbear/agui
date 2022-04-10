use agui_core::prelude::*;

#[derive(Debug, Default)]
pub struct Column {
    pub layout: Layout,

    pub spacing: Units,

    pub children: Vec<Widget>,
}

impl StatelessWidget for Column {
    fn build(&self, ctx: &mut BuildContext<()>) -> BuildResult {
        ctx.set_layout_type(LayoutType::Column {
            spacing: self.spacing,
        });

        ctx.set_layout(Layout::clone(&self.layout));

        (&self.children).into()
    }
}

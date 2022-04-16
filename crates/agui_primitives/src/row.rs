use agui_core::prelude::*;

#[derive(Debug, Default)]
pub struct Row {
    pub layout: Layout,

    pub spacing: Units,

    pub children: Vec<Widget>,
}

impl StatelessWidget for Row {
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        ctx.set_layout_type(LayoutType::Row {
            spacing: self.spacing,
        });

        ctx.set_layout(Layout::clone(&self.layout));

        (&self.children).into()
    }
}

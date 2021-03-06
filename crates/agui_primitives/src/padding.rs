use agui_core::{
    unit::{Layout, Margin, Sizing},
    widget::{BuildContext, BuildResult, StatelessWidget, Widget},
};

#[derive(Debug, Default)]
pub struct Padding {
    pub padding: Margin,

    pub child: Widget,
}

impl StatelessWidget for Padding {
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        ctx.set_layout(Layout {
            sizing: Sizing::Fill,

            margin: self.padding,

            ..Layout::default()
        });

        (&self.child).into()
    }
}

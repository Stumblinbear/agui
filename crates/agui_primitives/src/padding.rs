use agui_core::{
    unit::{Layout, Margin, Sizing},
    widget::{BuildContext, BuildResult, WidgetBuilder, WidgetRef},
};

#[derive(Debug, Default, PartialEq)]
pub struct Padding {
    pub padding: Margin,

    pub child: WidgetRef,
}

impl WidgetBuilder for Padding {
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        ctx.set_layout(Layout {
            sizing: Sizing::Fill,

            margin: self.padding,

            ..Layout::default()
        });

        (&self.child).into()
    }
}

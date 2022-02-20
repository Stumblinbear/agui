use agui_core::{
    unit::{Layout, Margin, Sizing},
    widget::{BuildResult, WidgetBuilder, BuildContext, WidgetRef},
};
use agui_macros::Widget;

#[derive(Default, Widget)]
pub struct Padding {
    pub padding: Margin,

    pub child: WidgetRef,
}

impl WidgetBuilder for Padding {
    fn build(&self, ctx: &mut BuildContext) -> BuildResult {
        ctx.set_layout(
            Layout {
                sizing: Sizing::Fill,
                
                margin: self.padding,
                
                ..Layout::default()
            }
            .into(),
        );

        (&self.child).into()
    }
}

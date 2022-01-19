use agui_core::{
    unit::{Layout, Margin},
    widget::{BuildResult, WidgetBuilder, WidgetContext, WidgetRef},
};
use agui_macros::Widget;

#[derive(Default, Widget)]
pub struct Padding {
    pub padding: Margin,

    pub child: WidgetRef,
}

impl WidgetBuilder for Padding {
    fn build(&self, ctx: &mut WidgetContext) -> BuildResult {
        ctx.set_layout(
            Layout {
                margin: self.padding,
                ..Layout::default()
            }
            .into(),
        );

        (&self.child).into()
    }
}

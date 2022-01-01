use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::Margin,
    widget::{BuildResult, WidgetBuilder, WidgetRef},
};
use agui_macros::Widget;

#[derive(Default, Widget)]
pub struct Padding {
    pub padding: Margin,

    pub child: WidgetRef,
}

impl WidgetBuilder for Padding {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
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
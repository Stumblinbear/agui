use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::{Margin, Sizing},
    widget::{BuildResult, WidgetBuilder, WidgetRef},
};
use agui_macros::Widget;

#[derive(Default, Widget)]
#[widget(layout = "column")]
pub struct Padding {
    pub padding: Margin,

    pub child: WidgetRef,
}

impl WidgetBuilder for Padding {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
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

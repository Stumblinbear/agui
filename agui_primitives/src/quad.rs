use agui_core::{
    context::WidgetContext,
    layout::LayoutRef,
    unit::Color,
    widget::{BuildResult, WidgetImpl, WidgetRef},
};
use agui_macros::Widget;

#[derive(Debug, Default, Widget)]
pub struct Quad {
    pub layout: LayoutRef,
    pub color: Color,

    pub clip: bool,
    pub child: WidgetRef,
}

impl WidgetImpl for Quad {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        ctx.set_layout(LayoutRef::clone(&self.layout));

        (&self.child).into()
    }
}

use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::Color,
    widget::{BuildResult, WidgetImpl, WidgetRef},
    Ref,
};
use agui_macros::Widget;

#[derive(Clone, Default)]
pub struct QuadStyle {
    pub color: Color,
}

#[derive(Default, Widget)]
pub struct Quad {
    pub layout: Ref<Layout>,
    pub clip: bool,

    pub style: QuadStyle,

    pub child: WidgetRef,
}

impl WidgetImpl for Quad {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        ctx.set_layout(Ref::clone(&self.layout));

        (&self.child).into()
    }
}

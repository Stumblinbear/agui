use agui_core::{
    context::WidgetContext,
    unit::{Color, Layout, Ref, Shape},
    widget::{BuildResult, WidgetBuilder, WidgetRef},
};
use agui_macros::Widget;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DrawableStyle {
    pub color: Color,
    pub opacity: f32,
}

#[derive(Default, Widget)]
pub struct Drawable {
    pub layout: Ref<Layout>,

    pub shape: Shape,
    pub style: Option<DrawableStyle>,

    pub child: WidgetRef,
}

impl WidgetBuilder for Drawable {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        ctx.set_layout(Ref::clone(&self.layout));

        (&self.child).into()
    }
}

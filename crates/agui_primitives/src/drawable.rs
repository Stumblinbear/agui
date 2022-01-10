use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::{Color, Shape},
    widget::{BuildResult, WidgetBuilder, WidgetRef},
    Ref,
};
use agui_macros::Widget;

#[derive(Clone, Default)]
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

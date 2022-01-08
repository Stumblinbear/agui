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
}

#[derive(Widget)]
pub struct Drawable {
    pub layout: Ref<Layout>,

    pub shape: Shape,
    pub style: Option<DrawableStyle>,

    pub child: WidgetRef,
}

impl Default for Drawable {
    fn default() -> Self {
        Self {
            layout: Ref::default(),

            shape: Shape::default(),
            style: Option::default(),

            child: WidgetRef::default(),
        }
    }
}

impl WidgetBuilder for Drawable {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        ctx.set_layout(Ref::clone(&self.layout));

        (&self.child).into()
    }
}

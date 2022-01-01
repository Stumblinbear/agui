use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::Color,
    widget::{BuildResult, WidgetBuilder, WidgetRef},
    Ref,
};
use agui_macros::Widget;

#[derive(Clone, Default)]
pub struct QuadStyle {
    pub color: Color,
}

#[derive(Widget)]
pub struct Quad {
    pub layout: Ref<Layout>,
    pub style: Option<QuadStyle>,

    pub radius: f32,
    pub clip: bool,

    pub child: WidgetRef,
}

impl Default for Quad {
    fn default() -> Self {
        Self {
            layout: Ref::default(),
            style: Option::default(),

            radius: 0.0,
            clip: true,

            child: WidgetRef::default(),
        }
    }
}

impl WidgetBuilder for Quad {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        ctx.set_layout(Ref::clone(&self.layout));

        (&self.child).into()
    }
}

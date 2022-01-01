use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::Color,
    widget::{BuildResult, WidgetBuilder, WidgetRef},
    Ref,
};
use agui_macros::Widget;

pub enum Shape {
    Rect { radius: f32 },
    Circle,
}

impl Default for Shape {
    fn default() -> Self {
        Shape::Rect { radius: 0.0 }
    }
}

#[derive(Clone, Default)]
pub struct DrawableStyle {
    pub color: Color,
}

#[derive(Widget)]
pub struct Drawable {
    pub layout: Ref<Layout>,
    pub style: Option<DrawableStyle>,

    pub shape: Shape,
    pub clip: bool,

    pub child: WidgetRef,
}

impl Default for Drawable {
    fn default() -> Self {
        Self {
            layout: Ref::default(),
            style: Option::default(),

            shape: Shape::default(),
            clip: true,

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

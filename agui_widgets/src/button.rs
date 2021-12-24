use agui_core::{
    context::WidgetContext,
    layout::{Layout, LayoutRef},
    unit::{Color, Sizing},
    widget::{BuildResult, WidgetImpl, WidgetRef},
};
use agui_macros::{build, Widget};
use agui_primitives::Quad;

use crate::state::MousePosition;

#[derive(Debug, Default, Widget)]
#[widget(layout = "row")]
pub struct Button {
    pub layout: LayoutRef,

    pub color: Color,

    pub child: WidgetRef,
}

impl WidgetImpl for Button {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        let _hovering = ctx.computed(|ctx| {
            let mouse = ctx.get_state::<MousePosition>();

            let mouse_pos = mouse.read();

            mouse_pos.x > 50.0
        });

        ctx.set_layout(LayoutRef::clone(&self.layout));

        build! {
            Quad {
                layout: Layout {
                    sizing: Sizing::Fill
                },
                color: Color::White,
                child: (&self.child).into()
            }
        }
    }
}

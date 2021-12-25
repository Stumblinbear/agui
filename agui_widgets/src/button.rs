use agui_core::{
    context::WidgetContext,
    layout::{Layout, LayoutRef},
    unit::{Color, Sizing},
    widget::{BuildResult, WidgetImpl, WidgetRef},
};
use agui_macros::{build, Widget};
use agui_primitives::Quad;

use crate::state::hovering::Hovering;

#[derive(Debug, Default, Widget)]
#[widget(layout = "row")]
pub struct Button {
    pub layout: LayoutRef,

    pub color: Color,

    pub child: WidgetRef,
}

impl WidgetImpl for Button {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        let hovering = ctx.computed(|ctx| {
            if let Some(hovering) = ctx.get_global::<Hovering>() {
                hovering.read().is_hovering(ctx)
            } else {
                false
            }
        });

        ctx.set_layout(LayoutRef::clone(&self.layout));

        build! {
            Quad {
                layout: Layout {
                    sizing: Sizing::Fill
                },
                color: if hovering {
                    Color::Green
                }else{
                    Color::White
                },
                child: (&self.child).into()
            }
        }
    }
}

use agui_core::{
    context::WidgetContext,
    layout::{Layout, LayoutRef},
    unit::{Color, Sizing},
    widget::{BuildResult, WidgetImpl, WidgetRef},
};
use agui_macros::{build, Widget};
use agui_primitives::{Quad, QuadStyle};

use crate::state::hovering::Hovering;

#[derive(Clone, Default)]
pub struct ButtonStyle {
    pub normal: QuadStyle,
    pub hover: QuadStyle,
    pub pressed: QuadStyle,
}

#[derive(Default, Widget)]
#[widget(layout = "row")]
pub struct Button {
    pub layout: LayoutRef,

    pub style: ButtonStyle,

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
                style: if hovering {
                    self.style.hover.clone()
                }else{
                    self.style.normal.clone()
                },
                child: (&self.child).into()
            }
        }
    }
}

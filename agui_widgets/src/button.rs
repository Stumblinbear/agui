use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::{Color, Sizing},
    widget::{BuildResult, WidgetBuilder, WidgetRef},
    Ref,
};
use agui_macros::{build, Widget};
use agui_primitives::{Quad, QuadStyle};

use crate::state::hovering::Hovering;

#[derive(Clone)]
pub struct ButtonStyle {
    pub normal: QuadStyle,
    pub hover: QuadStyle,
    pub pressed: QuadStyle,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            normal: QuadStyle {
                color: Color::White,
            },

            hover: QuadStyle {
                color: Color::Green,
            },

            pressed: QuadStyle::default(),
        }
    }
}

#[derive(Default, Widget)]
#[widget(layout = "row")]
pub struct Button {
    pub layout: Ref<Layout>,

    pub style: ButtonStyle,

    pub child: WidgetRef,
}

impl WidgetBuilder for Button {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        let hovering = ctx.computed(|ctx| {
            if let Some(hovering) = ctx.get_global::<Hovering>() {
                hovering.read().is_hovering(ctx)
            } else {
                false
            }
        });

        ctx.set_layout(Ref::clone(&self.layout));

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

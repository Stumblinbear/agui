use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::{Callback, Color, Sizing},
    widget::{BuildResult, WidgetBuilder, WidgetRef},
    Ref,
};
use agui_macros::{build, Widget};
use agui_primitives::{Quad, QuadStyle};

use crate::state::{
    hovering::Hovering,
    mouse::{Mouse, MouseButtonState},
};

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum ButtonState {
    Normal,
    Hover,
    Pressed,
}

#[derive(Default, Widget)]
#[widget(layout = "row")]
pub struct Button {
    pub layout: Ref<Layout>,

    pub style: ButtonStyle,

    pub child: WidgetRef,

    pub on_pressed: Callback,
}

impl WidgetBuilder for Button {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        ctx.set_layout(Ref::clone(&self.layout));

        let state = ctx.computed(|ctx| {
            if let Some(hovering) = ctx.get_global::<Hovering>() {
                if let Some(mouse) = ctx.get_global::<Mouse>() {
                    if hovering.read().is_hovering(ctx) {
                        if mouse.read().button.left == MouseButtonState::Pressed {
                            return ButtonState::Pressed;
                        } else {
                            return ButtonState::Hover;
                        }
                    }
                }
            }

            ButtonState::Normal
        });

        build! {
            Quad {
                layout: Layout {
                    sizing: Sizing::Fill
                },
                style: match state {
                    ButtonState::Normal => self.style.normal.clone(),
                    ButtonState::Hover => self.style.normal.clone(),
                    ButtonState::Pressed => self.style.normal.clone(),
                },
                child: &self.child
            }
        }
    }
}

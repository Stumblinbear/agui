use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::{Callback, Color},
    widget::{BuildResult, WidgetBuilder, WidgetRef},
    Ref,
};
use agui_macros::{build, Widget};
use agui_primitives::{Quad, QuadStyle};

use crate::state::{
    hovering::Hovering,
    mouse::{Mouse, MouseButtonState},
    theme::{Style, Theme},
};

#[derive(Clone)]
pub struct ButtonStyle {
    pub normal: QuadStyle,
    pub hover: QuadStyle,
    pub pressed: QuadStyle,
}

impl Style for ButtonStyle {}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            normal: QuadStyle {
                color: Color::White,
            },

            hover: QuadStyle {
                color: Color::LightGray,
            },

            pressed: QuadStyle {
                color: Color::DarkGray,
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ButtonState {
    Normal,
    Hover,
    Pressed,
}

#[derive(Default, Widget)]
#[widget(layout = "row")]
pub struct Button {
    pub layout: Ref<Layout>,

    pub style: Option<ButtonStyle>,

    pub child: WidgetRef,

    pub on_pressed: Callback<()>,
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

        // We init the state, instead of using `get_state`, because we don't want to react to
        // these changes, only keep track of them.
        let last_state = ctx.init_state(|| state);

        if *last_state.read() == ButtonState::Pressed {
            if let ButtonState::Pressed = state {
            } else {
                self.on_pressed.emit(());
            }
        }

        if *last_state.read() != state {
            *last_state.write() = state;
        }

        let style = Theme::resolve(ctx, &self.style);

        build! {
            Quad {
                layout: Ref::clone(&self.layout),
                style: match state {
                    ButtonState::Normal => style.normal.into(),
                    ButtonState::Hover => style.hover.into(),
                    ButtonState::Pressed => style.pressed.into(),
                },
                child: &self.child
            }
        }
    }
}

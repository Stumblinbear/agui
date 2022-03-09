use agui_core::{
    canvas::{clipping::Clip, paint::Paint},
    unit::{Color, Layout},
    widget::{callback::Callback, BuildContext, BuildResult, WidgetBuilder, WidgetRef},
};
use agui_macros::Widget;

use crate::{
    plugins::hovering::HoveringExt,
    state::mouse::{Mouse, MouseButtonState},
};

#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub normal: Color,
    pub disabled: Color,
    pub hover: Color,
    pub pressed: Color,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            normal: Color::White,
            disabled: Color::LightGray,
            hover: Color::LightGray,
            pressed: Color::DarkGray,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ButtonState {
    Normal,
    Disabled,
    Pressed,
}

#[derive(Default, Widget)]
pub struct Button {
    pub layout: Layout,
    pub style: Option<ButtonStyle>,
    pub clip: Option<Clip>,

    pub on_pressed: Callback<()>,

    pub child: WidgetRef,
}

impl WidgetBuilder for Button {
    fn build(&self, ctx: &mut BuildContext) -> BuildResult {
        ctx.set_layout(Layout::clone(&self.layout));

        ctx.use_effect({
            let on_pressed = self.on_pressed.clone();

            move |ctx| {
                if let Some(mouse) = ctx.try_use_global::<Mouse>() {
                    let mut state = ctx.init_state(|| ButtonState::Normal);

                    if mouse.button.left == MouseButtonState::Pressed {
                        if ctx.is_hovering() && *state != ButtonState::Pressed {
                            state.set(ButtonState::Pressed);
                        }
                    } else if *state == ButtonState::Pressed {
                        state.set(ButtonState::Normal);

                        if ctx.is_hovering() {
                            on_pressed.emit(());
                        }
                    }
                }
            }
        });

        {
            let state = ctx.use_state(|| ButtonState::Normal);
            let style = self.style.clone().unwrap_or_default();

            ctx.on_draw(move |canvas| {
                let color = match *state {
                    ButtonState::Normal => style.normal,
                    ButtonState::Disabled => style.disabled,
                    ButtonState::Pressed => style.pressed,
                };

                let brush = canvas.new_brush(Paint { color });

                canvas.draw_rect(brush);
            });
        }

        (&self.child).into()
    }
}

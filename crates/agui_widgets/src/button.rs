use agui_core::{
    canvas::{clipping::Clip, paint::Paint},
    unit::{Callback, Color, Layout, Ref},
    widget::{BuildResult, WidgetBuilder, WidgetContext, WidgetRef},
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
    Hover,
    Pressed,
}

#[derive(Default, Widget)]
pub struct Button {
    pub layout: Ref<Layout>,

    pub style: Option<ButtonStyle>,

    pub child: WidgetRef,

    pub clip: Option<Clip>,

    pub on_pressed: Callback<()>,
}

impl WidgetBuilder for Button {
    fn build(&self, ctx: &mut WidgetContext) -> BuildResult {
        ctx.set_layout(Ref::clone(&self.layout));

        let state = ctx.computed(|ctx| {
            if let Some(mouse) = ctx.try_use_global::<Mouse>() {
                if ctx.is_hovering() {
                    if mouse.read().button.left == MouseButtonState::Pressed {
                        return ButtonState::Pressed;
                    } else {
                        return ButtonState::Hover;
                    }
                }
            }

            ButtonState::Normal
        });

        // We init the state, instead of using `use_state`, because we don't want to react to
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

        ctx.on_draw({
            let style = self.style.clone().unwrap_or_default();

            move |canvas| {
                let color = match state {
                    ButtonState::Normal => style.normal,
                    ButtonState::Disabled => style.disabled,
                    ButtonState::Hover => style.hover,
                    ButtonState::Pressed => style.pressed,
                };

                let brush = canvas.new_brush(Paint { color });

                canvas.draw_rect(brush);
            }
        });

        (&self.child).into()
    }
}

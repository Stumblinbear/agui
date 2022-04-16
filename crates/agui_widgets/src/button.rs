use agui_core::prelude::*;

use crate::GestureDetector;

#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub normal: Color,
    pub disabled: Color,
    pub hovered: Color,
    pub pressed: Color,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            normal: Color::White,
            disabled: Color::LightGray,
            hovered: Color::LightGray,
            pressed: Color::DarkGray,
        }
    }
}

#[derive(Debug, Default)]
pub struct ButtonState {
    pressed: bool,
    hovered: bool,
    disabled: bool,
}

#[derive(Debug, Default)]
pub struct Button {
    pub layout: Layout,
    pub style: Option<ButtonStyle>,

    pub on_pressed: Callback<()>,

    pub child: Widget,
}

impl StatefulWidget for Button {
    type State = ButtonState;

    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        ctx.set_layout(Layout::clone(&self.layout));

        let color = {
            let state = ctx.get_state();

            let style = self.style.clone().unwrap_or_default();

            if state.disabled {
                style.disabled
            } else if state.pressed {
                style.pressed
            } else if state.hovered {
                style.hovered
            } else {
                style.normal
            }
        };

        ctx.on_draw(move |ctx, canvas| {
            let brush = canvas.new_brush(Paint {
                color,
                ..Paint::default()
            });

            canvas.draw_rect(brush);
        });

        let on_hover = ctx.callback::<bool, _>(|ctx, arg| {
            if ctx.get_state().hovered != *arg {
                ctx.set_state(|state| {
                    state.hovered = *arg;
                })
            }
        });

        let on_pressed = ctx.callback::<bool, _>({
            let on_pressed = self.on_pressed.clone();

            move |ctx, arg| {
                let state = ctx.get_state_mut();

                if state.pressed && !arg {
                    on_pressed.emit(());
                }

                ctx.set_state(|state| {
                    state.pressed = *arg;
                })
            }
        });

        ctx.key(
            Key::single(),
            GestureDetector {
                on_hover,
                on_pressed,
                child: self.child.clone(),
            }
            .into(),
        )
        .into()
    }
}

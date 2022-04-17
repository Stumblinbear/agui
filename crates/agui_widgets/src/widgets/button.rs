use agui_core::prelude::*;
use agui_macros::build;

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

        ctx.on_draw(move |ctx, canvas| {
            let style = ctx.style.clone().unwrap_or_default();

            let color = if ctx.state.disabled {
                style.disabled
            } else if ctx.state.pressed {
                style.pressed
            } else if ctx.state.hovered {
                style.hovered
            } else {
                style.normal
            };

            let brush = canvas.new_brush(Paint {
                color,
                ..Paint::default()
            });

            canvas.draw_rect(brush);
        });

        let on_hover = ctx.callback::<bool, _>(|ctx, arg| {
            if ctx.state.hovered != *arg {
                ctx.set_state(|state| {
                    state.hovered = *arg;
                })
            }
        });

        let on_pressed = ctx.callback::<bool, _>(|ctx, arg| {
            if ctx.state.pressed && !arg {
                ctx.emit(ctx.on_pressed, ());
            }

            ctx.set_state(|state| {
                state.pressed = *arg;
            })
        });

        build! {
            ctx.key(
                Key::single(),
                GestureDetector {
                    on_hover,
                    on_pressed,
                    child: self.child.clone(),
                }.into(),
            )
        }
    }
}

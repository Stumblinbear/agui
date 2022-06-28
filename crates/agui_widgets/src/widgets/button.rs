use agui_core::{
    callback::Callback,
    manager::context::Context,
    render::canvas::paint::Paint,
    unit::{Color, Key, Layout},
    widget::{BuildContext, BuildResult, Widget, WidgetBuilder},
};

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
            normal: Color::from_rgb((1.0, 1.0, 1.0)),
            disabled: Color::from_rgb((0.3, 0.3, 0.3)),
            hovered: Color::from_rgb((0.7, 0.7, 0.7)),
            pressed: Color::from_rgb((0.5, 0.5, 0.5)),
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

impl WidgetBuilder for Button {
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

            canvas.draw_rect(&Paint {
                color,
                ..Paint::default()
            });
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
                ctx.on_pressed.call(());
            }

            ctx.set_state(|state| {
                state.pressed = *arg;
            })
        });

        ctx.key(
            Key::single(),
            GestureDetector {
                on_hover,
                on_pressed,

                child: self.child.clone(),

                ..Default::default()
            }
            .into(),
        )
        .into()
    }
}

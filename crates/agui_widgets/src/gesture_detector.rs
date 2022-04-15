use agui_core::prelude::*;

use crate::{
    plugins::{event::EventPluginContextExt, global::GlobalPluginExt},
    state::mouse::{MouseButton, MouseButtonState, MousePos},
};

#[derive(Debug, Default)]
pub struct GestureDetector {
    pub on_hover: Callback<bool>,
    pub on_pressed: Callback<bool>,

    pub child: Widget,
}

#[derive(Debug, Default)]
pub struct GestureState {
    hovering: bool,
    pressed: bool,
}

impl StatefulWidget for GestureDetector {
    type State = GestureState;

    fn build(&self, ctx: &mut BuildContext<GestureState>) -> BuildResult {
        ctx.set_layout(Layout {
            sizing: Sizing::Fill,
            ..Default::default()
        });

        ctx.listen_to::<MousePos, _>({
            let on_hover = self.on_hover.clone();

            move |ctx, event| {
                if let Some(rect) = ctx.get_rect() {
                    if let Some(pos) = **event {
                        if rect.contains((pos.x as f32, pos.y as f32)) {
                            let state = ctx.get_state_mut();

                            if !state.hovering {
                                state.hovering = true;

                                on_hover.emit(true);
                            }

                            return;
                        }
                    }
                }

                let state = ctx.get_state_mut();

                if state.hovering {
                    state.hovering = false;

                    on_hover.emit(false);
                }
            }
        });

        ctx.listen_to::<MouseButton, _>({
            let on_pressed = self.on_pressed.clone();

            move |ctx, event| {
                let pos = ctx.get_global::<MousePos>();
                let pos = pos.borrow();

                let is_hovering = if let Some(rect) = ctx.get_rect() {
                    if let Some(pos) = **pos {
                        rect.contains((pos.x as f32, pos.y as f32))
                    } else {
                        false
                    }
                } else {
                    false
                };

                println!("{:?}", pos);

                let state = ctx.get_state_mut();

                if is_hovering {
                    if let MouseButton::Left(btn) = event {
                        match btn {
                            MouseButtonState::Pressed => {
                                state.pressed = true;

                                on_pressed.emit(true);
                            }

                            MouseButtonState::Released => {
                                if state.pressed {
                                    state.pressed = false;

                                    on_pressed.emit(false);
                                }
                            }
                        }
                    }
                }
            }
        });

        (&self.child).into()
    }
}

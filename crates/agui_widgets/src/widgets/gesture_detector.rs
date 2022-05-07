use agui_core::{
    callback::Callback,
    manager::{context::Context, widget::Widget},
    unit::{Layout, Sizing},
    widget::{BuildContext, BuildResult, StatefulWidget},
};

use crate::{
    plugins::{event::EventPluginContextExt, global::GlobalPluginExt},
    state::mouse::{MouseButton, MouseButtonState, MousePos},
};

#[derive(Debug, Default)]
pub struct GestureDetector {
    pub on_hover: Callback<bool>,
    pub on_pressed: Callback<bool>,

    pub is_focused: bool,
    pub on_focus: Callback<bool>,

    pub child: Widget,
}

#[derive(Debug, Default)]
pub struct GestureState {
    hovering: bool,
    pressed: bool,
    focused: bool,
}

impl StatefulWidget for GestureDetector {
    type State = GestureState;

    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        // Allow us to carry over the focused state through rebuilds
        ctx.state.focused = self.is_focused;

        ctx.set_layout(Layout {
            sizing: Sizing::Fill,
            ..Default::default()
        });

        if self.on_hover.is_some() {
            ctx.listen_to::<MousePos, _>(|ctx, event| {
                if let Some(rect) = ctx.get_rect() {
                    if let Some(pos) = **event {
                        if rect.contains((pos.x as f32, pos.y as f32)) {
                            if !ctx.state.hovering {
                                ctx.state.hovering = true;

                                ctx.on_hover.call(true);
                            }

                            return;
                        }
                    }
                }

                if ctx.state.hovering {
                    ctx.state.hovering = false;

                    ctx.on_hover.call(false);
                }
            });
        }

        if self.on_pressed.is_some() {
            ctx.listen_to::<MouseButton, _>(|ctx, event| {
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

                if is_hovering {
                    if let MouseButton::Left(btn) = event {
                        match btn {
                            MouseButtonState::Pressed => {
                                ctx.state.pressed = true;

                                ctx.on_pressed.call(true);
                            }

                            MouseButtonState::Released => {
                                if ctx.state.pressed {
                                    ctx.state.pressed = false;

                                    ctx.on_pressed.call(false);
                                }
                            }
                        }
                    }
                }
            });
        }

        if self.on_focus.is_some() {
            ctx.listen_to::<MouseButton, _>(|ctx, event| {
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

                if let MouseButton::Left(MouseButtonState::Pressed) = event {
                    if is_hovering {
                        ctx.state.focused = true;

                        ctx.on_focus.call(true);
                    } else if ctx.state.focused {
                        ctx.state.focused = false;

                        ctx.on_focus.call(false);
                    }
                }
            });
        }

        (&self.child).into()
    }
}

use agui_core::prelude::*;

use crate::{plugins::event::EventPluginContextExt, state::mouse::MouseButton};

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

#[derive(Debug, Default)]
pub struct ButtonState {
    pressed: bool,
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

    fn build(&self, ctx: &mut BuildContext<Self::State>) -> BuildResult {
        ctx.set_layout(Layout::clone(&self.layout));

        ctx.listen_to::<MouseButton, _>(|ctx, event| {
            
        });

        // let mouse = ctx.global::<Mouse>();
        // let state = ctx.new_state(ButtonState::default);

        // ctx.use_state::<ButtonState>()
        //     .try_use_global::<Mouse>()
        //     .with(self.on_pressed.clone())

        // ctx.effect(|ctx| {
        //     let is_pressed = state.map(ctx, |state| state.pressed);

        // let state_ref = state.as_ref(ctx);

        // if mouse.button.left == MouseButtonState::Pressed {
        //     if ctx.is_hovering() && *state != state.pressed {
        //         state.set(ctx, |state| {
        //             state.pressed = true;
        //         });
        //     }
        // } else if *state == ButtonState::Pressed {
        //     state.pressed = false;

        //     if ctx.is_hovering() {
        //         self.on_pressed.emit(());
        //     }
        // }
        // });

        // let state = ctx.use_state::<ButtonState>().get();

        // ctx.on_draw({
        //     let style = self.style.clone().unwrap_or_default();

        //     move |canvas| {
        //         let color = match *state {
        //             ButtonState::Normal => style.normal,
        //             ButtonState::Disabled => style.disabled,
        //             ButtonState::Pressed => style.pressed,
        //         };

        //         let brush = canvas.new_brush(Paint {
        //             color,
        //             ..Paint::default()
        //         });

        //         canvas.draw_rect(brush);
        //     }
        // });

        (&self.child).into()
    }
}

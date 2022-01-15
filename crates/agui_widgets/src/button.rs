use agui_core::{
    context::WidgetContext,
    unit::{Callback, Color, Layout, Ref, Sizing},
    widget::{BuildResult, WidgetBuilder, WidgetRef},
};
use agui_macros::{build, Widget};
use agui_primitives::{Drawable, DrawableStyle};

use crate::{
    plugins::hovering::Hovering,
    state::{
        mouse::{Mouse, MouseButtonState},
        theme::StyleExt,
    },
};

#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub normal: DrawableStyle,
    pub disabled: DrawableStyle,
    pub hover: DrawableStyle,
    pub pressed: DrawableStyle,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            normal: DrawableStyle {
                color: Color::White,
                opacity: 1.0,
            },

            disabled: DrawableStyle {
                color: Color::LightGray,
                opacity: 1.0,
            },

            hover: DrawableStyle {
                color: Color::LightGray,
                opacity: 1.0,
            },

            pressed: DrawableStyle {
                color: Color::DarkGray,
                opacity: 1.0,
            },
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

    pub on_pressed: Callback<()>,
}

impl WidgetBuilder for Button {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        ctx.set_layout(Ref::clone(&self.layout));

        let state = ctx.computed(|ctx| {
            if let Some(hovering) = ctx.try_use_global::<Hovering>() {
                if let Some(mouse) = ctx.try_use_global::<Mouse>() {
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

        let style: ButtonStyle = self.style.resolve(ctx);

        build! {
            Drawable {
                // We need to pass through sizing parameters so that the Drawable can react to child size if necessary,
                // but also fill the Button if the button itself is set to a non-Auto size.
                layout: Layout {
                    sizing: self.layout.try_get().map_or(Sizing::default(), |layout| layout.sizing)
                },

                style: match state {
                    ButtonState::Normal => style.normal.into(),
                    ButtonState::Disabled => style.disabled.into(),
                    ButtonState::Hover => style.hover.into(),
                    ButtonState::Pressed => style.pressed.into(),
                },

                child: &self.child
            }
        }
    }
}

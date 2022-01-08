use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::{Callback, Color, Shape, Sizing},
    widget::{BuildResult, WidgetBuilder, WidgetRef},
    Ref,
};
use agui_macros::{build, Widget};
use agui_primitives::{Drawable, DrawableStyle};

use crate::state::{
    hovering::Hovering,
    mouse::{Mouse, MouseButtonState},
    theme::{Style, Theme},
};

#[derive(Clone)]
pub struct ButtonStyle {
    pub normal: DrawableStyle,
    pub hover: DrawableStyle,
    pub pressed: DrawableStyle,
}

impl Style for ButtonStyle {}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            normal: DrawableStyle {
                color: Color::White,
            },

            hover: DrawableStyle {
                color: Color::LightGray,
            },

            pressed: DrawableStyle {
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
pub struct Button {
    pub layout: Ref<Layout>,

    pub style: Option<ButtonStyle>,

    pub child: WidgetRef,

    pub on_pressed: Callback<()>,
}

impl WidgetBuilder for Button {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        ctx.set_clipping(
            Shape::RoundedRect {
                top_left: 4.0,
                top_right: 4.0,
                bottom_right: 4.0,
                bottom_left: 4.0,
            }
            .into(),
        );

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

        let style = Theme::resolve(ctx, &self.style);

        build! {
            Drawable {
                // We need to pass through sizing parameters so that the Drawable can react to child size if necessary,
                // but also fill the Button if the button itself is set to a non-Auto size.
                layout: Layout {
                    sizing: self.layout.try_get().map_or(Sizing::default(), |layout| layout.sizing)
                },
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

use std::time::{Duration, Instant};

use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::{Color, Sizing},
    widget::{BuildResult, WidgetBuilder},
    Ref,
};
use agui_macros::{build, Widget};
use agui_primitives::{Drawable, DrawableStyle, FontDescriptor, Text};

use crate::{
    plugins::{hovering::Hovering, timer::TimerExt},
    state::{
        keyboard::KeyboardInput,
        mouse::{Mouse, MouseButtonState},
        theme::StyleExt,
    },
};

const CURSOR_BLINK_SECS: f32 = 0.5;

#[derive(Default, Clone)]
pub struct TextInputStateStyle {
    pub drawable: DrawableStyle,

    pub placeholder_color: Color,
    pub text_color: Color,
}

#[derive(Clone)]
pub struct TextInputStyle {
    pub normal: TextInputStateStyle,
    pub disabled: TextInputStateStyle,
    pub hover: TextInputStateStyle,
    pub focused: TextInputStateStyle,
}

impl Default for TextInputStyle {
    fn default() -> Self {
        Self {
            normal: TextInputStateStyle {
                drawable: DrawableStyle {
                    color: Color::White,
                    opacity: 1.0,
                },

                placeholder_color: Color::DarkGray,
                text_color: Color::Black,
            },

            disabled: TextInputStateStyle {
                drawable: DrawableStyle {
                    color: Color::DarkGray,
                    opacity: 1.0,
                },

                placeholder_color: Color::DarkGray,
                text_color: Color::Black,
            },

            hover: TextInputStateStyle {
                drawable: DrawableStyle {
                    color: Color::LightGray,
                    opacity: 1.0,
                },

                placeholder_color: Color::DarkGray,
                text_color: Color::Black,
            },

            focused: TextInputStateStyle {
                drawable: DrawableStyle {
                    color: Color::White,
                    opacity: 1.0,
                },

                placeholder_color: Color::DarkGray,
                text_color: Color::Black,
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TextInputState {
    Normal,
    Disabled,
    Hover,
    Focused,
}

impl Default for TextInputState {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CursorState {
    Shown,
    Hidden,
}

#[derive(Default, Widget)]
pub struct TextInput {
    pub layout: Ref<Layout>,

    pub style: Option<TextInputStyle>,

    pub font: FontDescriptor,
    pub placeholder: String,
}

impl WidgetBuilder for TextInput {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        ctx.set_layout(Ref::clone(&self.layout));

        let input_state = ctx.computed(|ctx| {
            let last_input_state = *ctx.init_state(TextInputState::default).read();

            if let Some(hovering) = ctx.try_use_global::<Hovering>() {
                if let Some(mouse) = ctx.try_use_global::<Mouse>() {
                    let is_hovering = hovering.read().is_hovering(ctx);
                    let is_pressed = mouse.read().button.left == MouseButtonState::Pressed;

                    if is_pressed {
                        return if is_hovering {
                            TextInputState::Focused
                        } else {
                            TextInputState::Normal
                        };
                    }

                    // If we're not already focused, and we're hovering the widget
                    if last_input_state != TextInputState::Focused && is_hovering {
                        return TextInputState::Hover;
                    }
                }
            }

            if last_input_state == TextInputState::Focused {
                return TextInputState::Focused;
            }

            TextInputState::Normal
        });

        let last_input_state = ctx.init_state(TextInputState::default);

        if *last_input_state.read() != input_state {
            *last_input_state.write() = input_state;
        }

        let style: TextInputStyle = self.style.resolve(ctx);

        // Since the value reacts to keyboard inputs, we use a computed value
        let mut value = ctx.computed(|ctx| {
            let input_value = ctx.init_state::<String, _>(|| "".into());

            let input_state = *ctx.init_state(TextInputState::default).read();

            if let Some(keyboard_input) = ctx.try_use_global::<KeyboardInput>() {
                if input_state == TextInputState::Focused {
                    match keyboard_input.read().0 {
                        // Backspace character
                        '\u{8}' => {
                            if input_value.read().len() > 0 {
                                input_value.write().pop();
                            }
                        }

                        ch => input_value.write().push(ch),
                    }
                }
            }

            let input_value = input_value.read();

            String::clone(&input_value)
        });

        let cursor_state = ctx.computed(|ctx| {
            // Listen to the input state so we can start blinking
            let input_state = *ctx.use_state(TextInputState::default).read();

            if input_state != TextInputState::Focused {
                return CursorState::Hidden;
            }

            // Keep track of time so we can blink blonk the cursor
            let instant = *ctx.init_state(Instant::now).read();

            // Request an update in x seconds
            ctx.use_timeout(Duration::from_secs_f32(CURSOR_BLINK_SECS));

            // Alternate between shown and hidden
            if instant.elapsed().as_secs_f32() % (CURSOR_BLINK_SECS * 2.0) > CURSOR_BLINK_SECS {
                CursorState::Hidden
            } else {
                CursorState::Shown
            }
        });

        if !value.is_empty() && cursor_state == CursorState::Shown {
            value.push('|');
        }

        let input_state_style = match input_state {
            TextInputState::Normal => style.normal,
            TextInputState::Disabled => style.disabled,
            TextInputState::Hover => style.hover,
            TextInputState::Focused => style.focused,
        };

        build! {
            Drawable {
                // We need to pass through sizing parameters so that the Drawable can react to child size if necessary,
                // but also fill the Button if the button itself is set to a non-Auto size.
                layout: Layout {
                    sizing: self.layout.try_get().map_or(Sizing::default(), |layout| layout.sizing)
                },

                style: input_state_style.drawable.into(),

                child: {
                    if value.is_empty() {
                        Text::is(self.font, 32.0, String::clone(&self.placeholder)).color(input_state_style.placeholder_color)
                    }else{
                        Text::is(self.font, 32.0, String::clone(&value)).color(input_state_style.text_color)
                    }
                }
            }
        }
    }
}

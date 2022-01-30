use std::{
    borrow::Cow,
    time::{Duration, Instant},
};

use agui_core::{
    canvas::{font::FontStyle, paint::Paint},
    unit::{Color, Layout, Rect, Ref},
    widget::{BuildResult, WidgetBuilder, WidgetContext},
};
use agui_macros::Widget;

use crate::{
    plugins::{hovering::HoveringExt, timeout::TimeoutExt},
    state::{
        keyboard::KeyboardInput,
        mouse::{Mouse, MouseButtonState},
        theme::StyleExt,
    },
};

const CURSOR_BLINK_SECS: f32 = 0.5;

#[derive(Debug, Default, Clone)]
pub struct TextInputStateStyle {
    pub background_color: Color,
    pub cursor_color: Color,

    pub placeholder_color: Color,
    pub text_color: Color,
}

#[derive(Debug, Clone)]
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
                background_color: Color::White,
                cursor_color: Color::Black,

                placeholder_color: Color::DarkGray,
                text_color: Color::Black,
            },

            disabled: TextInputStateStyle {
                background_color: Color::DarkGray,
                cursor_color: Color::Black,

                placeholder_color: Color::DarkGray,
                text_color: Color::Black,
            },

            hover: TextInputStateStyle {
                background_color: Color::LightGray,
                cursor_color: Color::Black,

                placeholder_color: Color::DarkGray,
                text_color: Color::Black,
            },

            focused: TextInputStateStyle {
                background_color: Color::White,
                cursor_color: Color::Black,

                placeholder_color: Color::DarkGray,
                text_color: Color::Black,
            },
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum CursorState {
    Shown,
    Hidden,
}

#[derive(Default, Widget)]
pub struct TextInput {
    pub layout: Ref<Layout>,

    pub style: Option<TextInputStyle>,

    pub font: FontStyle,
    pub placeholder: Cow<'static, str>,
}

impl WidgetBuilder for TextInput {
    fn build(&self, ctx: &mut WidgetContext) -> BuildResult {
        ctx.set_layout(Ref::clone(&self.layout));

        let input_state = ctx.computed(|ctx| {
            let last_input_state = *ctx.init_state(TextInputState::default).read();

            let is_pressed =
                ctx.use_global(Mouse::default).read().button.left == MouseButtonState::Pressed;

            if is_pressed {
                return if ctx.is_hovering() {
                    TextInputState::Focused
                } else {
                    TextInputState::Normal
                };
            }

            if last_input_state == TextInputState::Focused {
                return TextInputState::Focused;
            } else if ctx.is_hovering() {
                return TextInputState::Hover;
            }

            TextInputState::Normal
        });

        let last_input_state = ctx.init_state(TextInputState::default);

        if *last_input_state.read() != input_state {
            *last_input_state.write() = input_state;
        }

        // Since the value reacts to keyboard inputs, we use a computed value
        let value = ctx.computed(|ctx| {
            let input_value = ctx.init_state::<String, _>(|| "".into());

            let input_state = *ctx.init_state(TextInputState::default).read();

            let keyboard_input = ctx.use_global(KeyboardInput::default);

            if input_state == TextInputState::Focused {
                match **keyboard_input.read() {
                    // Backspace character
                    '\u{8}' => {
                        if input_value.read().len() > 0 {
                            input_value.write().pop();
                        }
                    }

                    ch => input_value.write().push(ch),
                }
            }

            let input_value = input_value.read();

            String::clone(&input_value)
        });

        let cursor_state = ctx.computed(|ctx| {
            // Keep track of time so we can blink blonk the cursor
            let instant = *ctx.init_state(Instant::now).read();

            // Request an update in x seconds
            ctx.use_timeout(Duration::from_secs_f32(CURSOR_BLINK_SECS));

            let input_state = *ctx.init_state(TextInputState::default).read();

            if input_state != TextInputState::Focused {
                return CursorState::Hidden;
            }

            // Alternate between shown and hidden
            if instant.elapsed().as_secs_f32() % (CURSOR_BLINK_SECS * 2.0) > CURSOR_BLINK_SECS {
                CursorState::Hidden
            } else {
                CursorState::Shown
            }
        });

        let style: TextInputStyle = self.style.resolve(ctx);

        let input_state_style = match input_state {
            TextInputState::Normal => style.normal,
            TextInputState::Disabled => style.disabled,
            TextInputState::Hover => style.hover,
            TextInputState::Focused => style.focused,
        };

        ctx.on_draw({
            let background_color = input_state_style.background_color;

            let text_color = input_state_style.text_color;

            let cursor_color = input_state_style.cursor_color;

            let (is_placeholder, text) = if value.is_empty() {
                (true, self.placeholder.clone())
            } else {
                (false, value.into())
            };

            let font = self.font;

            move |canvas| {
                let bg_brush = canvas.new_brush(Paint {
                    color: background_color,
                });

                canvas.draw_rect(bg_brush);

                if cursor_state == CursorState::Shown {
                    let cursor_brush = canvas.new_brush(Paint {
                        color: cursor_color,
                    });

                    if is_placeholder {
                        canvas.draw_rect_at(
                            Rect {
                                x: 0.0,
                                y: 0.0,
                                width: font.size / 2.0,
                                height: font.size,
                            },
                            cursor_brush,
                        );
                    } else {
                        canvas.calc_text_size(font, text.clone(), move |canvas, size| {
                            canvas.draw_rect_at(
                                Rect {
                                    x: size.width,
                                    y: 0.0,
                                    width: font.size / 2.0,
                                    height: font.size,
                                },
                                cursor_brush,
                            );
                        });
                    }
                }

                let text_brush = canvas.new_brush(Paint { color: text_color });

                canvas.draw_text(text_brush, font, text.clone());
            }
        });

        BuildResult::None
    }
}

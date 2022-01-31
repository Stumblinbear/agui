use std::{
    borrow::Cow,
    time::{Duration, Instant},
};

use agui_core::{
    canvas::paint::Paint,
    font::FontStyle,
    unit::{Color, Layout, Point, Rect, Ref},
    widget::{BuildResult, WidgetBuilder, WidgetContext},
};
use agui_macros::Widget;

use crate::{
    plugins::{hovering::HoveringExt, timeout::TimeoutExt},
    state::{
        keyboard::{self, KeyCode, Keyboard, KeyboardInput},
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

        let cursor = ctx.use_state(Cursor::default);

        // Since the value reacts to keyboard inputs, we use a computed value
        let value = ctx.computed({
            let cursor = cursor.clone();

            move |ctx| {
                let input_value = ctx.init_state::<String, _>(|| "".into());
                let input_state = *ctx.init_state(TextInputState::default).read();

                let keyboard = ctx.use_global(Keyboard::default);
                let keyboard = keyboard.read();

                let instant = ctx.init_state(Instant::now);

                if input_state == TextInputState::Focused {
                    let index = cursor.read().index;

                    if let Some(input) = keyboard.input {
                        match input {
                            // Backspace character
                            '\u{8}' => {
                                if input_value.read().len() > 0 && index > 0 {
                                    cursor.write().index = index.saturating_sub(1);

                                    if index == input_value.read().len() {
                                        input_value.write().pop();
                                    } else {
                                        input_value.write().remove(index);
                                    }
                                }
                            }

                            // Delete character
                            '\u{7f}' => {
                                if input_value.read().len() > 0 && index < input_value.read().len()
                                {
                                    input_value.write().remove(index);
                                }
                            }

                            ch => {
                                cursor.write().index += 1;

                                input_value.write().insert(index, ch);
                            }
                        }
                    } else if keyboard.is_pressed(&KeyCode::Right) {
                        cursor.write().index = (index + 1).min(input_value.read().len());
                    } else if keyboard.is_pressed(&KeyCode::Left) {
                        cursor.write().index = (index.saturating_sub(1)).max(0);
                    }

                    *instant.write() = Instant::now();
                }

                let input_value = input_value.read().clone();

                String::clone(&input_value)
            }
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

        let glyphs = if value.is_empty() {
            Vec::default()
        } else if let Some(size) = ctx.get_size() {
            self.font.get_glyphs(size.into(), &value)
        } else {
            Vec::default()
        };

        ctx.on_draw({
            let font = self.font.clone();

            let placeholder = self.placeholder.clone();

            move |canvas| {
                let bg_brush = canvas.new_brush(Paint {
                    color: input_state_style.background_color,
                });

                canvas.draw_rect(bg_brush);

                if cursor_state == CursorState::Shown {
                    let cursor_brush = canvas.new_brush(Paint {
                        color: input_state_style.cursor_color,
                    });

                    if value.is_empty() {
                        canvas.draw_rect_at(
                            Rect {
                                x: 0.0,
                                y: 0.0,
                                width: 4.0,
                                height: font.size,
                            },
                            cursor_brush,
                        );
                    } else {
                        let pos = if let Some(g) = glyphs.get(cursor.read().index) {
                            Point {
                                x: g.glyph.position.x,
                                y: g.glyph.position.y,
                            }
                        } else if let Some(g) = glyphs.last() {
                            Point {
                                x: g.glyph.position.x + font.h_advance(g.glyph.id),
                                y: g.glyph.position.y,
                            }
                        } else {
                            Point { x: 0.0, y: 0.0 }
                        };

                        canvas.draw_rect_at(
                            Rect {
                                x: pos.x,
                                y: 0.0,
                                width: 4.0,
                                height: font.size,
                            },
                            cursor_brush,
                        );
                    }
                }

                if value.is_empty() {
                    let text_brush = canvas.new_brush(Paint {
                        color: input_state_style.placeholder_color,
                    });

                    canvas.draw_text(text_brush, font.clone(), placeholder.clone());
                } else {
                    let text_brush = canvas.new_brush(Paint {
                        color: input_state_style.text_color,
                    });

                    canvas.draw_text(text_brush, font.clone(), value.clone().into());
                }
            }
        });

        BuildResult::None
    }
}

#[derive(Debug, Default)]
struct Cursor {
    index: usize,
}

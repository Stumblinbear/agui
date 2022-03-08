use std::{
    borrow::Cow,
    time::{Duration, Instant},
};

use agui_core::{
    canvas::paint::Paint,
    font::FontStyle,
    unit::{Color, Layout, Point, Rect, Ref},
    widget::{BuildContext, BuildResult, WidgetBuilder},
};
use agui_macros::Widget;
use agui_primitives::edit::EditableText;

use crate::{
    plugins::{hovering::HoveringExt, timeout::TimeoutExt},
    state::{
        keyboard::{KeyCode, Keyboard},
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

#[derive(Widget)]
pub struct TextInput<S>
where
    S: EditableText + 'static,
{
    pub layout: Ref<Layout>,

    pub style: Option<TextInputStyle>,

    pub font: FontStyle,
    pub placeholder: Cow<'static, str>,
    pub value: S,
}

impl Default for TextInput<String> {
    fn default() -> Self {
        Self {
            layout: Ref::None,

            style: None,

            font: FontStyle::default(),
            placeholder: "".into(),
            value: "".into(),
        }
    }
}

impl<S> WidgetBuilder for TextInput<S>
where
    S: EditableText + 'static,
{
    fn build(&self, ctx: &mut BuildContext) -> BuildResult {
        ctx.set_layout(Ref::clone(&self.layout));

        ctx.init_state(|| self.value.clone());

        let input_state = ctx.computed(|ctx| {
            let last_input_state = ctx.init_state(TextInputState::default);

            let is_pressed =
                ctx.use_global(Mouse::default).button.left == MouseButtonState::Pressed;

            if is_pressed {
                return if ctx.is_hovering() {
                    TextInputState::Focused
                } else {
                    TextInputState::Normal
                };
            }

            if *last_input_state == TextInputState::Focused {
                return TextInputState::Focused;
            } else if ctx.is_hovering() {
                return TextInputState::Hover;
            }

            TextInputState::Normal
        });

        let mut last_input_state = ctx.init_state(TextInputState::default);

        if *last_input_state != input_state {
            last_input_state.set(input_state);
        }

        let value = ctx.computed(|ctx| {
            let input_state = ctx.init_state(TextInputState::default);
            let keyboard = ctx.use_global(Keyboard::default);

            let mut input_value = ctx.init_state::<S, _>(|| panic!("value not initialized"));
            let mut cursor = ctx.init_state(Cursor::default);

            if *input_state == TextInputState::Focused {
                if let Some(input) = keyboard.input {
                    match input {
                        // Backspace character
                        '\u{8}' => {
                            let grapheme_idx =
                                input_value.prev_grapheme_offset(cursor.string_index);

                            if let Some(idx) = grapheme_idx {
                                input_value.remove(idx..(cursor.string_index));

                                cursor.set(Cursor {
                                    string_index: idx,
                                    glyph_index: cursor.glyph_index - 1,
                                });
                            }
                        }

                        // Delete character
                        '\u{7f}' => {
                            let grapheme_idx =
                                input_value.next_grapheme_offset(cursor.string_index);

                            if let Some(idx) = grapheme_idx {
                                input_value.remove((cursor.string_index)..idx);
                            }
                        }

                        ch => {
                            input_value.insert(cursor.string_index, ch);

                            let grapheme_idx =
                                input_value.next_grapheme_offset(cursor.string_index);

                            if let Some(idx) = grapheme_idx {
                                cursor.set(Cursor {
                                    string_index: idx,
                                    glyph_index: cursor.glyph_index + 1,
                                });
                            }
                        }
                    }
                } else if keyboard.is_pressed(&KeyCode::Right) {
                    let grapheme_idx = input_value.next_grapheme_offset(cursor.string_index);

                    if let Some(idx) = grapheme_idx {
                        cursor.set(Cursor {
                            string_index: idx,
                            glyph_index: cursor.glyph_index + 1,
                        });
                    }
                } else if keyboard.is_pressed(&KeyCode::Left) {
                    let grapheme_idx = input_value.prev_grapheme_offset(cursor.string_index);

                    if let Some(idx) = grapheme_idx {
                        cursor.set(Cursor {
                            string_index: idx,
                            glyph_index: cursor.glyph_index - 1,
                        });
                    }
                }

                ctx.set_state(Instant::now());
            }

            input_value.clone()
        });

        let cursor_state = ctx.computed(|ctx| {
            // Keep track of time so we can blink blonk the cursor
            let instant = ctx.use_state(Instant::now);

            // Request an update in x seconds
            ctx.use_timeout(Duration::from_secs_f32(CURSOR_BLINK_SECS));

            let input_state = ctx.init_state(TextInputState::default);

            if *input_state != TextInputState::Focused {
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
            self.font.get_glyphs(size.into(), value.as_str())
        } else {
            Vec::default()
        };

        {
            let font = self.font.clone();
            let placeholder = self.placeholder.clone();

            let cursor = ctx.use_state(Cursor::default);

            ctx.on_draw(move |canvas| {
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
                        let pos = if let Some(g) = glyphs.get(cursor.glyph_index) {
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

                    canvas.draw_text(text_brush, font.clone(), Cow::Owned(value.clone().into()));
                }
            });
        }

        BuildResult::None
    }
}

#[derive(Debug, Default, Copy, Clone)]
struct Cursor {
    string_index: usize,
    glyph_index: usize,
}

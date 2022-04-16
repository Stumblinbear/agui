use std::{
    borrow::Cow,
    time::{Duration, Instant},
};

use agui_core::prelude::*;
use agui_macros::build;
use agui_primitives::edit::EditableText;

use crate::{
    plugins::{event::EventPluginContextExt, timeout::TimeoutPluginExt},
    state::keyboard::{KeyCode, KeyState, KeyboardCharacter, KeyboardInput},
    GestureDetector,
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

#[derive(Debug, Default)]
pub struct TextInputState<S>
where
    S: EditableText + 'static,
{
    disabled: bool,
    hovered: bool,
    focused: bool,

    cursor: Cursor,

    value: S,
}

#[derive(Debug, Default)]
pub struct Cursor {
    shown: bool,
    instant: Option<Instant>,

    string_index: usize,
    glyph_index: usize,
}

#[derive(Debug)]
pub struct TextInput<S>
where
    S: EditableText + 'static,
{
    pub layout: Layout,

    pub style: TextInputStyle,

    pub font: FontStyle,
    pub placeholder: Cow<'static, str>,
    pub value: S,

    pub on_value: Callback<S>,
}

impl Default for TextInput<String> {
    fn default() -> Self {
        Self {
            layout: Layout::default(),

            style: TextInputStyle::default(),

            font: FontStyle::default(),
            placeholder: "".into(),
            value: "".into(),

            on_value: Callback::default(),
        }
    }
}

impl<S> StatefulWidget for TextInput<S>
where
    S: EditableText + Default + 'static,
{
    type State = TextInputState<S>;

    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        ctx.set_layout(Layout::clone(&self.layout));

        let on_focus = ctx.callback::<bool, _>(|ctx, arg| {
            if *arg {
                if !ctx.state.focused {
                    ctx.set_state(|state| {
                        state.focused = true;
                        state.cursor.shown = true;
                        state.cursor.instant = Some(Instant::now());
                    });
                }
            } else if ctx.state.focused {
                ctx.set_state(|state| {
                    state.focused = false;
                    state.cursor.shown = false;
                    state.cursor.instant = None;
                });
            }
        });

        let on_hover = ctx.callback::<bool, _>(|ctx, arg| {
            if ctx.state.hovered != *arg {
                ctx.set_state(|state| {
                    state.hovered = *arg;
                });
            }
        });

        if ctx.state.cursor.instant.is_some() {
            ctx.set_timeout(Duration::from_secs_f32(CURSOR_BLINK_SECS), |ctx, _| {
                ctx.set_state(|state| {
                    state.cursor.shown = !state.cursor.shown;
                });
            });
        }

        ctx.listen_to::<KeyboardInput, _>(|ctx, KeyboardInput(key, key_state)| {
            if *key_state == KeyState::Pressed {
                let input_value = &ctx.state.value;
                let cursor = &ctx.state.cursor;

                if *key == KeyCode::Right {
                    let grapheme_idx = input_value.next_grapheme_offset(cursor.string_index);

                    if let Some(idx) = grapheme_idx {
                        ctx.set_state(|state| {
                            state.cursor.string_index = idx;
                            state.cursor.glyph_index += 1;

                            state.cursor.shown = true;
                            state.cursor.instant = Some(Instant::now());
                        });
                    }
                } else if key == &KeyCode::Left {
                    let grapheme_idx = input_value.prev_grapheme_offset(cursor.string_index);

                    if let Some(idx) = grapheme_idx {
                        ctx.set_state(|state| {
                            state.cursor.string_index = idx;
                            state.cursor.glyph_index -= 1;

                            state.cursor.shown = true;
                            state.cursor.instant = Some(Instant::now());
                        });
                    }
                }
            }
        });

        ctx.listen_to::<KeyboardCharacter, _>(|ctx, KeyboardCharacter(input)| {
            let input_value = &mut ctx.state.value;
            let cursor = &ctx.state.cursor;

            match input {
                // Backspace character
                '\u{8}' => {
                    let grapheme_idx = input_value.prev_grapheme_offset(cursor.string_index);

                    if let Some(idx) = grapheme_idx {
                        input_value.remove(idx..(cursor.string_index));

                        ctx.set_state(|state| {
                            state.cursor.string_index = idx;
                            state.cursor.glyph_index -= 1;

                            state.cursor.shown = true;
                            state.cursor.instant = Some(Instant::now());
                        });
                    }
                }

                // Delete character
                '\u{7f}' => {
                    let grapheme_idx = input_value.next_grapheme_offset(cursor.string_index);

                    if let Some(idx) = grapheme_idx {
                        input_value.remove((cursor.string_index)..idx);

                        ctx.set_state(|state| {
                            state.cursor.shown = true;
                            state.cursor.instant = Some(Instant::now());
                        });
                    }
                }

                ch => {
                    input_value.insert(cursor.string_index, *ch);

                    let grapheme_idx = input_value.next_grapheme_offset(cursor.string_index);

                    if let Some(idx) = grapheme_idx {
                        ctx.set_state(|state| {
                            state.cursor.string_index = idx;
                            state.cursor.glyph_index += 1;

                            state.cursor.shown = true;
                            state.cursor.instant = Some(Instant::now());
                        });
                    }
                }
            }
        });

        // let style: TextInputStyle = self.style.resolve(ctx);

        ctx.on_draw(|ctx, canvas| {
            let input_state_style = if ctx.state.disabled {
                &ctx.style.disabled
            } else if ctx.state.hovered {
                &ctx.style.hover
            } else if ctx.state.focused {
                &ctx.style.focused
            } else {
                &ctx.style.normal
            };

            let bg_brush = canvas.new_brush(Paint {
                color: input_state_style.background_color,
                ..Paint::default()
            });

            canvas.draw_rect(bg_brush);

            if ctx.state.cursor.shown {
                let cursor_brush = canvas.new_brush(Paint {
                    color: input_state_style.cursor_color,
                    ..Paint::default()
                });

                if ctx.state.value.is_empty() {
                    canvas.draw_rect_at(
                        Rect {
                            x: 0.0,
                            y: 0.0,
                            width: 4.0,
                            height: ctx.font.size,
                        },
                        cursor_brush,
                    );
                } else {
                    let glyphs = ctx
                        .font
                        .get_glyphs(canvas.get_size().into(), ctx.state.value.as_str());

                    let pos = if let Some(g) = glyphs.get(ctx.state.cursor.glyph_index) {
                        Point {
                            x: g.glyph.position.x,
                            y: g.glyph.position.y,
                        }
                    } else if let Some(g) = glyphs.last() {
                        Point {
                            x: g.glyph.position.x + ctx.font.h_advance(g.glyph.id),
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
                            height: ctx.font.size,
                        },
                        cursor_brush,
                    );
                }
            }

            if ctx.state.value.is_empty() {
                let text_brush = canvas.new_brush(Paint {
                    color: input_state_style.placeholder_color,
                    ..Paint::default()
                });

                canvas.draw_text(text_brush, ctx.font.clone(), ctx.placeholder.clone());
            } else {
                let text_brush = canvas.new_brush(Paint {
                    color: input_state_style.text_color,
                    ..Paint::default()
                });

                canvas.draw_text(
                    text_brush,
                    ctx.font.clone(),
                    Cow::Owned(ctx.state.value.clone().into()),
                );
            }
        });

        build! {
            ctx.key(
                Key::single(),
                GestureDetector {
                    on_hover,

                    is_focused: ctx.state.focused,
                    on_focus,
                }.into()
            )
        }
    }
}

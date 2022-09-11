use std::{
    borrow::Cow,
    time::{Duration, Instant},
};

use agui_core::{
    callback::Callback,
    render::canvas::paint::Paint,
    unit::{Color, FontStyle, Key, Layout, Point, Rect},
    widget::{BuildContext, BuildResult, WidgetBuilder, WidgetContext},
};
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
                background_color: Color::from_rgb((1.0, 1.0, 1.0)),
                cursor_color: Color::from_rgb((0.0, 0.0, 0.0)),

                placeholder_color: Color::from_rgb((0.3, 0.3, 0.3)),
                text_color: Color::from_rgb((0.0, 0.0, 0.0)),
            },

            disabled: TextInputStateStyle {
                background_color: Color::from_rgb((0.3, 0.3, 0.3)),
                cursor_color: Color::from_rgb((0.0, 0.0, 0.0)),

                placeholder_color: Color::from_rgb((0.3, 0.3, 0.3)),
                text_color: Color::from_rgb((0.0, 0.0, 0.0)),
            },

            hover: TextInputStateStyle {
                background_color: Color::from_rgb((0.7, 0.7, 0.7)),
                cursor_color: Color::from_rgb((0.0, 0.0, 0.0)),

                placeholder_color: Color::from_rgb((0.3, 0.3, 0.3)),
                text_color: Color::from_rgb((0.0, 0.0, 0.0)),
            },

            focused: TextInputStateStyle {
                background_color: Color::from_rgb((1.0, 1.0, 1.0)),
                cursor_color: Color::from_rgb((0.0, 0.0, 0.0)),

                placeholder_color: Color::from_rgb((0.3, 0.3, 0.3)),
                text_color: Color::from_rgb((0.0, 0.0, 0.0)),
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

impl<S> WidgetBuilder for TextInput<S>
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
            let cursor = &ctx.state.cursor;

            match input {
                // Backspace character
                '\u{8}' => {
                    let grapheme_idx = ctx.state.value.prev_grapheme_offset(cursor.string_index);

                    if let Some(idx) = grapheme_idx {
                        ctx.state.value.remove(idx..(cursor.string_index));

                        ctx.on_value.call(ctx.state.value.clone());

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
                    let grapheme_idx = ctx.state.value.next_grapheme_offset(cursor.string_index);

                    if let Some(idx) = grapheme_idx {
                        ctx.state.value.remove((cursor.string_index)..idx);

                        ctx.on_value.call(ctx.state.value.clone());

                        ctx.set_state(|state| {
                            state.cursor.shown = true;
                            state.cursor.instant = Some(Instant::now());
                        });
                    }
                }

                ch => {
                    ctx.state.value.insert(cursor.string_index, ch.to_string());

                    let grapheme_idx = ctx.state.value.next_grapheme_offset(cursor.string_index);

                    if let Some(idx) = grapheme_idx {
                        ctx.on_value.call(ctx.state.value.clone());

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

        ctx.on_draw(|ctx, mut canvas| {
            let input_state_style = if ctx.state.disabled {
                &ctx.style.disabled
            } else if ctx.state.hovered {
                &ctx.style.hover
            } else if ctx.state.focused {
                &ctx.style.focused
            } else {
                &ctx.style.normal
            };

            canvas.draw_rect(&Paint {
                color: input_state_style.background_color,
                ..Paint::default()
            });

            if ctx.state.cursor.shown {
                let cursor_paint = &Paint {
                    color: input_state_style.cursor_color,
                    ..Paint::default()
                };

                if ctx.state.value.is_empty() {
                    canvas.draw_rect_at(
                        Rect {
                            x: 0.0,
                            y: 0.0,
                            width: 4.0,
                            height: ctx.font.size,
                        },
                        cursor_paint,
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
                        cursor_paint,
                    );
                }
            }

            if ctx.state.value.is_empty() {
                canvas.draw_text(
                    &Paint {
                        color: input_state_style.placeholder_color,
                        ..Paint::default()
                    },
                    ctx.font.clone(),
                    ctx.placeholder.clone(),
                );
            } else {
                canvas.draw_text(
                    &Paint {
                        color: input_state_style.text_color,
                        ..Paint::default()
                    },
                    ctx.font.clone(),
                    ctx.state.value.clone(),
                );
            }
        });

        ctx.key(
            Key::single(),
            GestureDetector {
                on_hover,

                is_focused: ctx.state.focused,
                on_focus,

                ..Default::default()
            },
        )
        .into()
    }
}

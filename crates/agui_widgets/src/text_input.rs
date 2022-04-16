use std::borrow::Cow;

use agui_core::prelude::*;
use agui_primitives::edit::EditableText;

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

#[derive(Debug, Default, Clone, Copy)]
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

#[derive(Debug, Default, Clone, Copy)]
pub struct Cursor {
    shown: bool,

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

        // let mouse = ctx.global::<Mouse>();
        // let keyboard = ctx.global::<Keyboard>();

        // let input_value = ctx.state::<S>().or(|| self.value.clone());
        // let input_state = ctx.state::<(TextInputState, TextInputState)>();
        // let cursor = ctx.state::<Cursor>();

        // (input_state, mouse)
        //     .stream()
        //     .map(|ctx, (input_state, mouse)| {
        //         if mouse.button.left == MouseButtonState::Pressed {
        //             return if ctx.is_hovering() {
        //                 TextInputState::Focused
        //             } else {
        //                 TextInputState::Normal
        //             };
        //         }

        //         if *input_state == TextInputState::Focused {
        //             return TextInputState::Focused;
        //         } else if ctx.is_hovering() {
        //             return TextInputState::Hover;
        //         }

        //         TextInputState::Normal
        //     })
        //     .get();

        // let value = (input_state, keyboard, cursor, input_value)
        //     .stream()
        //     .filter(|ctx, (input_state)| input_state == TextInputState::Focused)
        //     .map(|ctx, _| {
        //         if let Some(input) = keyboard.input {
        //             match input {
        //                 // Backspace character
        //                 '\u{8}' => {
        //                     let grapheme_idx =
        //                         input_value.prev_grapheme_offset(cursor.string_index);

        //                     if let Some(idx) = grapheme_idx {
        //                         input_value.remove(idx..(cursor.string_index));

        //                         cursor.set(Cursor {
        //                             string_index: idx,
        //                             glyph_index: cursor.glyph_index - 1,
        //                         });
        //                     }
        //                 }

        //                 // Delete character
        //                 '\u{7f}' => {
        //                     let grapheme_idx =
        //                         input_value.next_grapheme_offset(cursor.string_index);

        //                     if let Some(idx) = grapheme_idx {
        //                         input_value.remove((cursor.string_index)..idx);
        //                     }
        //                 }

        //                 ch => {
        //                     input_value.insert(cursor.string_index, ch);

        //                     let grapheme_idx =
        //                         input_value.next_grapheme_offset(cursor.string_index);

        //                     if let Some(idx) = grapheme_idx {
        //                         cursor.set(Cursor {
        //                             string_index: idx,
        //                             glyph_index: cursor.glyph_index + 1,
        //                         });
        //                     }
        //                 }
        //             }

        //             on_value.emit(input_value.clone());
        //         } else if keyboard.is_pressed(&KeyCode::Right) {
        //             let grapheme_idx = input_value.next_grapheme_offset(cursor.string_index);

        //             if let Some(idx) = grapheme_idx {
        //                 cursor.set(Cursor {
        //                     string_index: idx,
        //                     glyph_index: cursor.glyph_index + 1,
        //                 });
        //             }
        //         } else if keyboard.is_pressed(&KeyCode::Left) {
        //             let grapheme_idx = input_value.prev_grapheme_offset(cursor.string_index);

        //             if let Some(idx) = grapheme_idx {
        //                 cursor.set(Cursor {
        //                     string_index: idx,
        //                     glyph_index: cursor.glyph_index - 1,
        //                 });
        //             }
        //         }

        //         ctx.set_state(Instant::now());
        //     })

        // let value = {
        //     let on_value = self.on_value.clone();

        //     ctx.computed(move |ctx| {
        //         let input_state = ctx.init_state(TextInputState::default);
        //         let keyboard = ctx.use_global(Keyboard::default);

        //         let mut input_value = ctx.init_state::<S, _>(|| panic!("value not initialized"));
        //         let mut cursor = ctx.init_state(Cursor::default);

        //         if *input_state == TextInputState::Focused {
        //             if let Some(input) = keyboard.input {
        //                 match input {
        //                     // Backspace character
        //                     '\u{8}' => {
        //                         let grapheme_idx =
        //                             input_value.prev_grapheme_offset(cursor.string_index);

        //                         if let Some(idx) = grapheme_idx {
        //                             input_value.remove(idx..(cursor.string_index));

        //                             cursor.set(Cursor {
        //                                 string_index: idx,
        //                                 glyph_index: cursor.glyph_index - 1,
        //                             });
        //                         }
        //                     }

        //                     // Delete character
        //                     '\u{7f}' => {
        //                         let grapheme_idx =
        //                             input_value.next_grapheme_offset(cursor.string_index);

        //                         if let Some(idx) = grapheme_idx {
        //                             input_value.remove((cursor.string_index)..idx);
        //                         }
        //                     }

        //                     ch => {
        //                         input_value.insert(cursor.string_index, ch);

        //                         let grapheme_idx =
        //                             input_value.next_grapheme_offset(cursor.string_index);

        //                         if let Some(idx) = grapheme_idx {
        //                             cursor.set(Cursor {
        //                                 string_index: idx,
        //                                 glyph_index: cursor.glyph_index + 1,
        //                             });
        //                         }
        //                     }
        //                 }

        //                 on_value.emit(input_value.clone());
        //             } else if keyboard.is_pressed(&KeyCode::Right) {
        //                 let grapheme_idx = input_value.next_grapheme_offset(cursor.string_index);

        //                 if let Some(idx) = grapheme_idx {
        //                     cursor.set(Cursor {
        //                         string_index: idx,
        //                         glyph_index: cursor.glyph_index + 1,
        //                     });
        //                 }
        //             } else if keyboard.is_pressed(&KeyCode::Left) {
        //                 let grapheme_idx = input_value.prev_grapheme_offset(cursor.string_index);

        //                 if let Some(idx) = grapheme_idx {
        //                     cursor.set(Cursor {
        //                         string_index: idx,
        //                         glyph_index: cursor.glyph_index - 1,
        //                     });
        //                 }
        //             }

        //             ctx.set_state(Instant::now());
        //         }

        //         input_value.clone()
        //     })
        // };

        // let cursor_state = ctx.computed(|ctx| {
        //     // Keep track of time so we can blink blonk the cursor
        //     let instant = ctx.use_state(Instant::now);

        //     // Request an update in x seconds
        //     ctx.use_timeout(Duration::from_secs_f32(CURSOR_BLINK_SECS));

        //     let input_state = ctx.init_state(TextInputState::default);

        //     if *input_state != TextInputState::Focused {
        //         return CursorState::Hidden;
        //     }

        //     // Alternate between shown and hidden
        //     if instant.elapsed().as_secs_f32() % (CURSOR_BLINK_SECS * 2.0) > CURSOR_BLINK_SECS {
        //         CursorState::Hidden
        //     } else {
        //         CursorState::Shown
        //     }
        // });

        // let style: TextInputStyle = self.style.resolve(ctx);

        // let input_state_style = match input_state {
        //     TextInputState::Normal => style.normal,
        //     TextInputState::Disabled => style.disabled,
        //     TextInputState::Hover => style.hover,
        //     TextInputState::Focused => style.focused,
        // };

        // let glyphs = if value.is_empty() {
        //     Vec::default()
        // } else if let Some(size) = ctx.get_size() {
        //     self.font.get_glyphs(size.into(), value.as_str())
        // } else {
        //     Vec::default()
        // };

        ctx.on_draw(|ctx, canvas| {
            let state = ctx.get_state();

            let input_state_style = if state.disabled {
                &ctx.style.disabled
            } else if state.hovered {
                &ctx.style.hover
            } else if state.focused {
                &ctx.style.focused
            } else {
                &ctx.style.normal
            };

            let bg_brush = canvas.new_brush(Paint {
                color: input_state_style.background_color,
                ..Paint::default()
            });

            canvas.draw_rect(bg_brush);

            if state.cursor.shown {
                let cursor_brush = canvas.new_brush(Paint {
                    color: input_state_style.cursor_color,
                    ..Paint::default()
                });

                if state.value.is_empty() {
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
                        .get_glyphs(canvas.get_size().into(), state.value.as_str());

                    let pos = if let Some(g) = glyphs.get(state.cursor.glyph_index) {
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

            if state.value.is_empty() {
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

                canvas.draw_text(text_brush, ctx.font.clone(), Cow::Owned(state.value.clone().into()));
            }
        });

        BuildResult::None
    }
}

use std::time::{Duration, Instant};

use agui_core::{
    canvas::{font::FontStyle, painter::shape::RectPainter},
    unit::{Color, Key, Layout, Position, Ref, Sizing, Units},
    widget::{BuildResult, WidgetBuilder, WidgetContext, WidgetRef},
};
use agui_macros::{build, Widget};
use agui_primitives::{Font, Fonts, ScaleFont, Text};

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
                placeholder_color: Color::DarkGray,
                text_color: Color::Black,
            },

            disabled: TextInputStateStyle {
                background_color: Color::DarkGray,
                placeholder_color: Color::DarkGray,
                text_color: Color::Black,
            },

            hover: TextInputStateStyle {
                background_color: Color::LightGray,
                placeholder_color: Color::DarkGray,
                text_color: Color::Black,
            },

            focused: TextInputStateStyle {
                background_color: Color::White,
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

#[derive(Default, Widget)]
pub struct TextInput {
    pub layout: Ref<Layout>,

    pub style: Option<TextInputStyle>,

    pub font: FontStyle,
    pub placeholder: String,
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

        let style: TextInputStyle = self.style.resolve(ctx);

        let input_state_style = match input_state {
            TextInputState::Normal => style.normal,
            TextInputState::Disabled => style.disabled,
            TextInputState::Hover => style.hover,
            TextInputState::Focused => style.focused,
        };

        const FONT_HEIGHT: f32 = 32.0;
        const CURSOR_PADDING: f32 = 2.0;

        let (text, cursor_pos) = if value.is_empty() {
            (
                Text::is(self.font, FONT_HEIGHT, String::clone(&self.placeholder))
                    .color(input_state_style.placeholder_color),
                None,
            )
        } else {
            let text = Text::is(self.font, FONT_HEIGHT, String::clone(&value))
                .sizing(self.layout.get().sizing)
                .color(input_state_style.text_color);

            // If we know the widget's rect, calculate the glyphs so we know where to place the cursor
            // if let Some(rect) = ctx.use_rect() {
            //     let fonts = ctx.use_global(Fonts::default);
            //     let fonts = fonts.read();
            //     let fonts = fonts.get_fonts();

            //     let glyphs = text.get_glyphs(fonts, (rect.width, rect.height));

            //     if !glyphs.is_empty() {
            //         let g = glyphs.last().unwrap();

            //         let position = g.glyph.position;

            //         let mut pos_x = position.x;

            //         if let Some(font) = fonts.get(g.font_id.0) {
            //             pos_x += font.as_scaled(g.glyph.scale).h_advance(g.glyph.id);
            //         }

            //         // We have to subtract the rect height since morphorm doesn't let us stack widgets
            //         (
            //             text,
            //             Some((rect.x + pos_x, (-rect.height) + CURSOR_PADDING)),
            //         )
            //     } else {
            //         (text, None)
            //     }
            // } else {
            //     (text, None)
            // }

            (text, None)
        };

        ctx.set_painter(RectPainter {
            color: input_state_style.background_color,
        });

        build! {
            [
                text,

                if cursor_pos.is_some() && input_state == TextInputState::Focused {
                    // Key the cursor so its timer doesn't get reset with every change
                    ctx.key(Key::local(value), Cursor {
                        position: cursor_pos.unwrap(),
                        height: FONT_HEIGHT - CURSOR_PADDING * 2.0,

                        color: input_state_style.text_color,
                    }.into())
                }else{
                    WidgetRef::None
                },
            ]
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum CursorState {
    Shown,
    Hidden,
}

#[derive(Widget)]
pub struct Cursor {
    position: (f32, f32),
    height: f32,

    color: Color,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            position: (0.0, 0.0),
            height: 32.0,

            color: Color::default(),
        }
    }
}

impl WidgetBuilder for Cursor {
    fn build(&self, ctx: &mut WidgetContext) -> BuildResult {
        let cursor_state = ctx.computed(|ctx| {
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

        ctx.set_layout(build! {
            Layout {
                position: Position::Relative {
                    top: Units::Pixels(self.position.1),
                    left: Units::Pixels(self.position.0),
                    bottom: None,
                    right: None
                },
                sizing: Sizing::Axis {
                    width: 2.0,
                    height: self.height.into()
                }
            }
        });

        let mut rgba = self.color.as_rgba();

        rgba[3] = match cursor_state {
            CursorState::Shown => 1.0,
            CursorState::Hidden => 0.0,
        };

        ctx.set_painter(RectPainter { color: rgba.into() });

        BuildResult::None
    }
}

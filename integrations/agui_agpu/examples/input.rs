#![allow(clippy::needless_update)]

use agui::{
    font::Font,
    macros::{build, functional_widget},
    unit::{Color, Key, Layout, Sizing, Units},
    widget::{BuildContext, BuildResult, WidgetRef},
    widgets::{
        plugins::DefaultPluginsExt,
        primitives::{Column, Text},
        state::DefaultGlobalsExt,
        App, TextInput,
    },
};
use agui_agpu::UIProgram;

fn main() -> Result<(), agpu::BoxError> {
    let mut ui = UIProgram::new("agui input")?;

    ui.register_default_plugins();
    ui.register_default_globals();

    let deja_vu = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"))?;

    ui.set_root(build! {
        App {
            child: ExampleMain {
                font: deja_vu
            }
        }
    });

    ui.run()
}

#[functional_widget]
fn example_main(
    ctx: &mut BuildContext,
    font: Font,
    _color: Color,
    _child: WidgetRef,
) -> BuildResult {
    ctx.set_layout(Layout {
        sizing: Sizing::Fill,
        ..Layout::default()
    });

    let value = ctx.use_state(|| "".to_owned());

    let on_value = ctx.use_callback(|ctx, input: &String| {
        ctx.set_state(input.clone());
    });

    build! {
        Column {
            layout: Layout {
                sizing: Sizing::Axis {
                    width: Units::Stretch(1.0),
                    height: Units::Pixels(32.0),
                }
            },
            children: [
                ctx.key(Key::single(), TextInput {
                    layout: Layout {
                        sizing: Sizing::Axis {
                            width: Units::Stretch(1.0),
                            height: Units::Pixels(32.0),
                        }
                    },

                    font: font.styled(),
                    placeholder: "some text here",

                    on_value
                }.into()),
                Text {
                    font: font.styled().color(Color::White),
                    text: value.clone().into(),
                },
            ]
        }
    }
}

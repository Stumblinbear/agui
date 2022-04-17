#![allow(clippy::needless_update)]

use agui::{
    macros::{build, functional_widget},
    prelude::*,
    unit::{Color, Key, Layout, Sizing, Units},
    widget::{BuildContext, BuildResult, Widget},
    widgets::{
        plugins::DefaultPluginsExt,
        primitives::{Column, Text},
        App, TextInput,
    },
};
use agui_agpu::UIProgram;
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

fn main() -> Result<(), agpu::BoxError> {
    let filter = EnvFilter::from_default_env()
        .add_directive(LevelFilter::ERROR.into())
        .add_directive(format!("agui={}", LevelFilter::DEBUG).parse().unwrap());

    tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::time())
        .with_level(true)
        .with_thread_names(false)
        .with_target(true)
        .with_env_filter(filter)
        .init();

    let mut ui = UIProgram::new("agui input")?;

    ui.register_default_plugins();
    // ui.register_default_globals();

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

#[functional_widget(String)]
fn example_main(ctx: &mut BuildContext, font: Font, _color: Color, _child: Widget) -> BuildResult {
    ctx.set_layout(Layout {
        sizing: Sizing::Fill,
        ..Layout::default()
    });

    let on_value = ctx.callback::<String, _>(|ctx, input: &String| {
        ctx.set_state(|state| *state = input.clone());
    });

    build! {
        Column {
            layout: Layout {
                sizing: Sizing::Axis {
                    width: Units::Stretch(1.0),
                    height: Units::Stretch(1.0)
                },
                margin: Margin::center()
            },
            spacing: Units::Pixels(8.0),
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
                    text: ctx.state.clone(),
                },
            ]
        }
    }
}

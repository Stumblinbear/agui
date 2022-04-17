#![allow(clippy::needless_update)]
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    macros::{build, functional_widget},
    prelude::*,
    widgets::{
        plugins::DefaultPluginsExt,
        primitives::{Column, Padding, Text},
        App, Button,
    },
};
use agui_agpu::UIProgram;

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

    let mut ui = UIProgram::new("agui counter")?;

    ui.register_default_plugins();
    // ui.register_default_globals();

    let deja_vu = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"))?;

    ui.set_root(build! {
        App {
            child: CounterWidget {
                font: deja_vu.styled(),
            }
        }
    });

    ui.run()
}

#[functional_widget(i32)]
fn counter_widget(ctx: &mut BuildContext, font: FontStyle) -> BuildResult {
    let on_pressed = ctx.callback(|ctx, ()| {
        ctx.set_state(|state| {
            *state += 1;
        })
    });

    build! {
        Column {
            children: [
                Text { font: font.clone(), text: format!("clicked: {} times", ctx.get_state()).into() },
                ctx.key(Key::single(), Button {
                    layout: Layout {
                        sizing: Sizing::Axis {
                            width: 256.0,
                            height: 64.0,
                        },
                    },
                    child: Padding {
                        padding: Margin::All(10.0.into()),
                        child: Text { font, text: "A Button" }
                    },
                    on_pressed
                }.into())
            ]
        }
    }
}

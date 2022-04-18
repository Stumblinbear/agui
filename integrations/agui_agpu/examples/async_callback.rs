#![allow(clippy::needless_update)]
use std::{thread, time::Duration};

use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    macros::{build, functional_widget},
    prelude::*,
    widgets::{
        plugins::DefaultPluginsExt,
        primitives::{Column, Text},
        App,
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

    let mut ui = UIProgram::new("agui async callback")?;

    ui.register_default_plugins();
    // ui.register_default_globals();

    let deja_vu = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"))?;

    ui.set_root(build!(App {
        child: ExampleMain { font: deja_vu }
    }));

    ui.run()
}

#[functional_widget(usize)]
fn example_main(ctx: &mut BuildContext, font: Font, _color: Color, _child: Widget) -> BuildResult {
    ctx.set_layout(Layout {
        sizing: Sizing::Fill,
        ..Layout::default()
    });

    let callback = ctx.callback::<usize, _>(|ctx, num| {
        ctx.set_state(|state| *state = *num);
    });

    thread::spawn({
        let num = *ctx.state;

        move || {
            thread::sleep(Duration::from_millis(1000));

            callback.call(num + 1);
        }
    });

    build!(Column {
        layout: Layout {
            sizing: Sizing::Axis {
                width: Units::Stretch(1.0),
                height: Units::Auto
            },
        },
        children: [Text {
            font: font.styled().color(Color::White),
            text: format!("Called: {}", ctx.state).into(),
        }]
    })
}

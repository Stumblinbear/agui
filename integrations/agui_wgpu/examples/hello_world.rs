#![allow(clippy::needless_update)]

use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    prelude::*,
    widgets::{plugins::DefaultPluginsExt, primitives::Text, App},
};
use agui_wgpu::AguiProgram;

fn main() {
    let filter = EnvFilter::from_default_env()
        .add_directive(LevelFilter::ERROR.into())
        .add_directive(format!("agui={}", LevelFilter::INFO).parse().unwrap());

    tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::time())
        .with_level(true)
        .with_thread_names(false)
        .with_target(true)
        .with_env_filter(filter)
        .init();

    let mut ui = AguiProgram::new(
        "agui hello world",
        Size {
            width: 800.0,
            height: 600.0,
        },
    );

    ui.register_default_plugins();
    // ui.register_default_globals();

    let deja_vu = ui
        .load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"))
        .unwrap();

    ui.set_root(App {
        child: build! {
            Text {
                font: deja_vu.styled(),
                text: "Hello, world!",
            }
        },
    });

    ui.run()
}

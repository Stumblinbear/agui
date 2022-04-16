#![allow(clippy::needless_update)]

use agui::{
    macros::build,
    widgets::{plugins::DefaultPluginsExt, primitives::Text, App},
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

    let mut ui = UIProgram::new("agui hello world")?;

    ui.register_default_plugins();
    // ui.register_default_globals();

    let deja_vu = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"))?;

    ui.set_root(build! {
        App {
            child: Text {
                font: deja_vu.styled(),
                text: "Hello, world!"
            }
        }
    });

    ui.run()
}

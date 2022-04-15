#![allow(clippy::needless_update)]

use agui::{
    macros::build,
    widgets::{plugins::DefaultPluginsExt, primitives::Text, App},
};
use agui_agpu::UIProgram;

fn main() -> Result<(), agpu::BoxError> {
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

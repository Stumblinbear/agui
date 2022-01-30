#![allow(clippy::needless_update)]

use agui::{
    macros::build,
    unit::{Layout, Sizing, Units},
    widgets::{plugins::DefaultPluginsExt, state::DefaultGlobalsExt, App, TextInput},
};
use agui_agpu::UIProgram;

fn main() -> Result<(), agpu::BoxError> {
    let mut ui = UIProgram::new("agui input")?;

    ui.register_default_plugins();
    ui.register_default_globals();

    let deja_vu = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"))?;

    ui.set_root(build! {
        App {
            child: TextInput {
                layout: Layout {
                    sizing: Sizing::Axis {
                        width: Units::Stretch(1.0),
                        height: Units::Pixels(32.0),
                    }
                },

                font: deja_vu.styled(),
                placeholder: "some text here"
            }
        }
    });

    ui.run()
}

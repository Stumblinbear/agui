#![allow(clippy::needless_update)]

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
    let mut ui = UIProgram::new("agui counter")?;

    ui.register_default_plugins();
    // ui.register_default_globals();

    let deja_vu = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"))?;

    ui.set_root(App {
        child: build! {
            CounterWidget {
                font: deja_vu.styled(),
            }
        },
    });

    ui.run()
}

#[functional_widget]
fn counter_widget(ctx: &mut BuildContext<i32>, font: FontStyle) -> BuildResult {
    let on_pressed = ctx.callback(|ctx, ()| {
        ctx.set_state(|state| {
            *state += 1;
        })
    });

    build! {
        Column {
            children: [
                Text { font: font.clone(), text: format!("clicked: {} times", ctx.get_state()).into() },
                Button {
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
                }
            ]
        }
    }
}

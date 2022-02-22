#![allow(clippy::needless_update)]

use agui::{
    font::FontStyle,
    macros::{build, functional_widget},
    unit::{Callback, Layout, Margin, Sizing},
    widget::{BuildContext, BuildResult},
    widgets::{
        plugins::DefaultPluginsExt,
        primitives::{Column, Padding, Text},
        state::DefaultGlobalsExt,
        App, Button,
    },
};
use agui_agpu::UIProgram;

fn main() -> Result<(), agpu::BoxError> {
    let mut ui = UIProgram::new("agui counter")?;

    ui.register_default_plugins();
    ui.register_default_globals();

    let deja_vu = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"))?;

    ui.set_root(build! {
        App {
            child: CounterWidget {
                font: deja_vu.styled()
            }
        }
    });

    ui.run()
}

#[functional_widget]
fn counter_widget(ctx: &mut BuildContext, font: FontStyle) -> BuildResult {
    let num = ctx.use_state(|| 0);

    build! {
        Column {
            children: [
                Text { font: font.clone(), text: format!("clicked: {} times", num).into() },
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
                    on_pressed: Callback::from(move |()| {
                        *num.write() += 1;
                    })
                }
            ]
        }
    }
}

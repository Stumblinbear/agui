#![allow(clippy::needless_update)]
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    macros::{build, functional_widget},
    prelude::*,
    widgets::{
        plugins::DefaultPluginsExt,
        primitives::{Clip, Column, Padding, Text},
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

    let mut ui = UIProgram::new("agui clipping")?;

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

#[functional_widget]
fn example_main(ctx: &mut BuildContext, font: Font, _color: Color, _child: Widget) -> BuildResult {
    ctx.set_layout(Layout {
        sizing: Sizing::Fill,
        ..Layout::default()
    });

    build!(Column {
        layout: Layout {
            sizing: Sizing::Axis {
                width: Units::Stretch(1.0),
                height: Units::Stretch(1.0)
            },
            margin: Margin::center()
        },
        spacing: Units::Pixels(16.0),
        children: [Clip {
            shape: Shape::RoundedRect {
                top_left: 8.0,
                top_right: 8.0,
                bottom_right: 8.0,
                bottom_left: 8.0
            },
            child: Button {
                layout: Layout {
                    sizing: Sizing::Axis {
                        width: 256.0,
                        height: 64.0,
                    },
                },
                child: Padding {
                    padding: Margin::All(10.0.into()),
                    child: Text {
                        font: font
                            .styled()
                            .h_align(HorizontalAlign::Center)
                            .v_align(VerticalAlign::Center),
                        text: "A Button"
                    }
                },
                on_pressed: ctx.callback(|_ctx, ()| {
                    println!("Pressed 1");
                })
            }
        }]
    })
}

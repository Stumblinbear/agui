#![allow(clippy::needless_update)]
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    prelude::*,
    widgets::{
        primitives::{Clip, ColoredBox, Padding, SizedBox, Text},
        App,
    },
};
use agui_vello::AguiProgram;

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
        "agui clipping",
        Size {
            width: 800.0,
            height: 600.0,
        },
    );

    // ui.register_default_plugins();
    // ui.register_default_globals();

    let deja_vu = ui
        .load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"))
        .unwrap();

    ui.set_root(App {
        child: ExampleMain { font: deja_vu }.into(),
    });

    ui.run()
}

#[derive(StatelessWidget, PartialEq)]
struct ExampleMain {
    font: Font,
}

impl WidgetView for ExampleMain {
    fn layout(&self, _: &mut LayoutContext<Self>) -> LayoutResult {
        LayoutResult {
            layout_type: LayoutType::default(),

            layout: Layout {
                sizing: Sizing::Fill,
                ..Layout::default()
            },
        }
    }

    fn build(&self, ctx: &mut BuildContext<Self>) -> Children {
        Children::new(build! {
            ColoredBox {
                color: Color::from_rgb((1.0, 1.0, 1.0)),

                child: Padding {
                    padding: Margin::center(),
                    child: SizedBox {
                        width: 280.0,

                        child: Clip {
                            shape: Shape::RoundedRect {
                                top_left: 8.0,
                                top_right: 8.0,
                                bottom_right: 8.0,
                                bottom_left: 8.0
                            },

                            child: ColoredBox {
                                color: Color::from_rgb((0.75, 0.75, 0.75)),
                                child: Text {
                                    font: self.font.styled().color(Color::from_rgb((1.0, 0.0, 0.0))),
                                    text: "The Krabby Patty secret formula is one part love, two parts magic, and three parts secret ingredient."
                                },
                            }
                        }
                    }
                }
            }
        })
    }
}

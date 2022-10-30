#![allow(clippy::needless_update)]
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    prelude::*,
    widgets::{
        primitives::{Clip, Padding, Text},
        App, Button,
    },
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

    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        BuildResult::new(build! {Padding {
            padding: Margin::center(),
            child: Clip {
                rect: Rect {
                    width: 128.0,
                    height: 64.0
                },

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
                            font: self.font
                                .styled()
                                .h_align(HorizontalAlign::Center)
                                .v_align(VerticalAlign::Center),
                            text: "I am not   \nclipped properly"
                        }
                    },
                    on_pressed: ctx.callback(|_ctx, ()| {
                        println!("Pressed 1");
                    })
                }
            }
        }})
    }
}

#![allow(clippy::needless_update)]
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    macros::{build, functional_widget},
    prelude::*,
    widgets::{
        plugins::DefaultPluginsExt,
        primitives::{Builder, Column, Padding, Spacing, Text},
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

    let mut ui = UIProgram::new("agui title screen")?;

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

    build! {
        Column {
            layout: Layout {
                sizing: Sizing::Axis {
                    width: Units::Stretch(1.0),
                    height: Units::Stretch(1.0)
                },
                margin: Margin::center()
            },
            spacing: Units::Pixels(16.0),
            children: [
                Text {
                    font: font.styled().h_align(HorizontalAlign::Center).size(64.0).color(Color::White),
                    text: "A Title".into()
                },
                Spacing::vertical(32.0.into()),
                Button {
                    layout: Layout {
                        sizing: Sizing::Axis {
                            width: 256.0,
                            height: 64.0,
                        },
                    },
                    child: Padding {
                        padding: Margin::All(10.0.into()),
                        child: Text {
                            font: font.styled().h_align(HorizontalAlign::Center).v_align(VerticalAlign::Center),
                            text: "A Button"
                        }
                    },
                    on_pressed: ctx.callback(|_ctx, ()| {
                        println!("Pressed 1");
                    })
                },
                Button {
                    layout: Layout {
                        sizing: Sizing::Axis {
                            width: 400.0,
                            height: 50.0,
                        },
                    },
                    child: Padding {
                        padding: Margin::All(10.0.into()),
                        child: Text {
                            font: font.styled().h_align(HorizontalAlign::Center).v_align(VerticalAlign::Center),
                            text: "Another Button"
                        }
                    },
                    on_pressed: ctx.callback(|_ctx, ()| {
                        println!("Pressed 2");
                    })
                },
                Button {
                    layout: Layout {
                        sizing: Sizing::Axis {
                            width: 150.0,
                            height: 100.0,
                        },
                    },
                    child: Padding {
                        padding: Margin::All(10.0.into()),
                        child: Text {
                            font: font.styled().h_align(HorizontalAlign::Left).v_align(VerticalAlign::Bottom),
                            text: "Also a Button"
                        }
                    },
                    on_pressed: ctx.callback(|_ctx, ()| {
                        println!("Pressed 3");
                    })
                },
                Builder::new(move |ctx| {
                    // let theme = ctx.init_state(|| {
                    //     let mut theme = Theme::new();

                    //     theme.set(ButtonStyle {
                    //         normal: Color::Red,
                    //         hover: Color::Green,
                    //         pressed: Color::Blue,
                    //     });

                    //     theme
                    // });

                    // theme.provide(ctx);

                    build! {
                        Button {
                            layout: Layout {
                                sizing: Sizing::Axis {
                                    width: 256.0,
                                    height: 64.0,
                                },
                            },
                            child: Padding {
                                padding: Margin::All(10.0.into()),
                                child: Text {
                                    font: font.styled().color(Color::White).h_align(HorizontalAlign::Right).v_align(VerticalAlign::Bottom),
                                    text: "Beuton"
                                }
                            },
                            on_pressed: ctx.callback(|_ctx, ()| {
                                println!("Pressed 4");
                            })
                        }
                    }
                })
            ]
        }
    }
}

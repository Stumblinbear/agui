#![allow(clippy::needless_update)]

use agpu::Features;
use agui::{
    layout::Layout,
    macros::build,
    unit::{Callback, Color, Sizing},
    widgets::{
        plugins::{hovering::HoveringPlugin, provider::ProviderExt},
        primitives::{Builder, Column, QuadStyle, Text},
        state::{
            hovering::Hovering,
            keyboard::{Keyboard, KeyboardInput},
            mouse::{Mouse, Scroll},
            theme::Theme,
        },
        App, Button, ButtonStyle,
    },
};
use agui_agpu::UI;

fn main() -> Result<(), agpu::BoxError> {
    let program = agpu::GpuProgram::builder("agui widgets")
        .with_gpu_features(Features::POLYGON_MODE_LINE)
        .with_framerate(60.0)
        .build()?;

    let mut ui = UI::with_default(&program);

    ui.get_context().init_plugin::<HoveringPlugin>();

    ui.get_context().init_global::<Keyboard>();
    ui.get_context().init_global::<KeyboardInput>();

    ui.get_context().init_global::<Mouse>();
    ui.get_context().init_global::<Scroll>();

    ui.get_context().init_global::<Hovering>();

    let dejavu_font = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"));

    ui.set_root(build! {
        App {
            child: Column {
                children: [
                    Button {
                        layout: Layout {
                            sizing: Sizing::Set {
                                width: 100,
                                height: 100
                            }
                        },
                        child: Text::is(dejavu_font, 16.0, "A Button".into()),
                        on_pressed: Callback::from(|()| {
                            println!("Pressed 1");
                        })
                    },
                    Button {
                        layout: Layout {
                            sizing: Sizing::Set {
                                width: 200,
                                height: 100
                            }
                        },
                        child: Text::is(dejavu_font, 32.0, "A Button".into()),
                        on_pressed: Callback::from(|()| {
                            println!("Pressed 2");
                        })
                    },
                    Builder::new(move |ctx| {
                        let mut theme = Theme::new();

                        theme.set(ButtonStyle {
                            normal: QuadStyle {
                                color: Color::Red,
                            },

                            hover: QuadStyle {
                                color: Color::Green,
                            },

                            pressed: QuadStyle {
                                color: Color::Blue,
                            },
                        });

                        let theme = ctx.set_state(theme);

                        theme.provide(ctx);

                        build!{
                            Button {
                                layout: Layout {
                                    sizing: Sizing::Set {
                                        width: 50,
                                        height: 200
                                    }
                                },
                                child: Text::is(dejavu_font, 16.0, "A Button".into()),
                                on_pressed: Callback::from(|()| {
                                    println!("Pressed 3");
                                })
                            }
                        }
                    })
                ]
            }
        }
    });

    ui.run(program)
}

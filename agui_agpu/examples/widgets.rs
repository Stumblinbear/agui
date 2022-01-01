#![allow(clippy::needless_update)]

use agpu::Features;
use agui::{
    macros::build,
    unit::{Callback, Color, Margin},
    widgets::{
        plugins::{hovering::HoveringPlugin, provider::ProviderExt},
        primitives::{Builder, Column, DrawableStyle, Padding, Text},
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

    ui.get_context().init_plugin(HoveringPlugin::default);

    ui.get_context().init_global(Keyboard::default);
    ui.get_context().init_global(KeyboardInput::default);

    ui.get_context().init_global(Mouse::default);
    ui.get_context().init_global(Scroll::default);

    ui.get_context().init_global(Hovering::default);

    let dejavu_font = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"));

    ui.set_root(build! {
        App {
            child: Column {
                children: [
                    Text::is(dejavu_font, 32.0, "A Title".into()),
                    Button {
                        child: Padding {
                            padding: Margin::All(10.0.into()),
                            child: Text::is(dejavu_font, 32.0, "A Button".into())
                        },
                        on_pressed: Callback::from(|()| {
                            println!("Pressed 1");
                        })
                    },
                    Button {
                        child: Padding {
                            padding: Margin::All(10.0.into()),
                            child: Text::is(dejavu_font, 32.0, "A Button".into())
                        },
                        on_pressed: Callback::from(|()| {
                            println!("Pressed 1");
                        })
                    },
                    Button {
                        child: Text::is(dejavu_font, 32.0, "A Button".into()),
                        on_pressed: Callback::from(|()| {
                            println!("Pressed 2");
                        })
                    },
                    Builder::new(move |ctx| {
                        let theme = ctx.init_state(|| {
                            let mut theme = Theme::new();

                            theme.set(ButtonStyle {
                                normal: DrawableStyle {
                                    color: Color::Red,
                                },

                                hover: DrawableStyle {
                                    color: Color::Green,
                                },

                                pressed: DrawableStyle {
                                    color: Color::Blue,
                                },
                            });

                            theme
                        });

                        theme.provide(ctx);

                        build!{
                            Button {
                                child: Padding {
                                    padding: Margin::All(10.0.into()),
                                    child: Text::is(dejavu_font, 32.0, "A Button".into())
                                },
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

#![allow(clippy::needless_update)]

use agui::{
    canvas::{clipping::Clip, font::FontStyle},
    macros::{build, functional_widget},
    unit::{Callback, Color, Layout, Margin, Sizing, Units},
    widget::{BuildResult, WidgetContext, WidgetRef},
    widgets::{
        plugins::{provider::ProviderExt, DefaultPluginsExt},
        primitives::{Builder, Column, Padding, Spacing},
        state::theme::Theme,
        App, Button, ButtonStyle,
    },
};
use agui_agpu::UIProgram;

fn main() -> Result<(), agpu::BoxError> {
    let mut ui = UIProgram::new("agui widgets")?;

    ui.register_default_plugins();

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
fn example_main(
    ctx: &mut WidgetContext,
    font: FontStyle,
    _color: Color,
    _child: WidgetRef,
) -> BuildResult {
    ctx.set_layout(
        Layout {
            sizing: Sizing::Fill,
            ..Layout::default()
        }
        .into(),
    );

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
                // Text::is(font, 64.0, "A Title".into()).color(Color::White),
                Spacing::vertical(32.0.into()),
                Button {
                    child: Padding {
                        padding: Margin::All(10.0.into()),
                        // child: Text::is(font, 32.0, "A Button".into())
                    },
                    on_pressed: Callback::from(|()| {
                        println!("Pressed 1");
                    })
                },
                Button {
                    child: Padding {
                        padding: Margin::All(10.0.into()),
                        // child: Text::is(font, 32.0, "Another Button".into())
                    },
                    on_pressed: Callback::from(|()| {
                        println!("Pressed 1");
                    })
                },
                Button {
                    clip: Clip::Hard.into(),
                    child: Padding {
                        padding: Margin::All(10.0.into()),
                        // child: Text::is(font, 32.0, "Also a Button".into())
                    },
                    on_pressed: Callback::from(|()| {
                        println!("Pressed 2");
                    })
                },
                Builder::new(move |ctx| {
                    let theme = ctx.init_state(|| {
                        let mut theme = Theme::new();

                        theme.set(ButtonStyle {
                            normal: Color::Red,
                            hover: Color::Green,
                            pressed: Color::Blue,
                        });

                        theme
                    });

                    theme.provide(ctx);

                    build! {
                        Button {
                            child: Padding {
                                padding: Margin::All(10.0.into()),
                                // child: Text::is(font, 32.0, "Beuton".into()).color(Color::White)
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
}

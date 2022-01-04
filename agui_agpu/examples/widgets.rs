#![allow(clippy::needless_update)]

use agpu::Features;
use agui::{
    context::WidgetContext,
    layout::Layout,
    macros::{build, functional_widget},
    unit::{Callback, Color, Margin, Sizing, Units},
    widget::{BuildResult, WidgetRef},
    widgets::{
        plugins::{hovering::HoveringPlugin, provider::ProviderExt},
        primitives::{Builder, Column, DrawableStyle, FontId, Padding, Spacing, Text},
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
        .with_gpu_features(
            Features::POLYGON_MODE_LINE
                | Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                | Features::VERTEX_WRITABLE_STORAGE,
        )
        // .with_framerate(f32::MAX)
        .build()?;

    let mut ui = UI::with_default(&program);

    ui.get_context().init_plugin(HoveringPlugin::default);

    ui.get_context().init_global(Keyboard::default);
    ui.get_context().init_global(KeyboardInput::default);

    ui.get_context().init_global(Mouse::default);
    ui.get_context().init_global(Scroll::default);

    ui.get_context().init_global(Hovering::default);

    ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"));

    ui.set_root(build! {
        App {
            child: ExampleMain::default()
        }
    });

    ui.run(program)
}

#[functional_widget]
fn example_main(ctx: &WidgetContext, _color: Color, _child: WidgetRef) -> BuildResult {
    ctx.set_layout(
        Layout {
            sizing: Sizing::Fill,
            ..Layout::default()
        }
        .into(),
    );

    let default_font = FontId(0);

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
                Text::is(default_font, 64.0, "A Title".into()).color(Color::White),
                Spacing::vertical(32.0.into()),
                Button {
                    child: Padding {
                        padding: Margin::All(10.0.into()),
                        child: Text::is(default_font, 32.0, "A Button".into())
                    },
                    on_pressed: Callback::from(|()| {
                        println!("Pressed 1");
                    })
                },
                Button {
                    child: Padding {
                        padding: Margin::All(10.0.into()),
                        child: Text::is(default_font, 32.0, "Another Button".into())
                    },
                    on_pressed: Callback::from(|()| {
                        println!("Pressed 1");
                    })
                },
                Button {
                    child: Padding {
                        padding: Margin::All(10.0.into()),
                        child: Text::is(default_font, 32.0, "Also a Button".into())
                    },
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
                                child: Text::is(default_font, 32.0, "Beuton".into()).color(Color::White)
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

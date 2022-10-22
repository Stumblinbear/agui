#![allow(clippy::needless_update)]
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    prelude::*,
    widgets::{
        plugins::DefaultPluginsExt,
        primitives::{Column, Padding, Text},
        App, Button,
    },
};
use agui_agpu::AguiProgram;

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

    let mut ui = AguiProgram::new("agui counter")?;

    ui.register_default_plugins();
    // ui.register_default_globals();

    let deja_vu = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"))?;

    ui.set_root(App {
        child: CounterWidget { font: deja_vu }.into(),
    });

    ui.run()
}

#[derive(StatefulWidget, PartialEq)]
struct CounterWidget {
    font: Font,
}

impl WidgetState for CounterWidget {
    type State = usize;

    fn create_state(&self) -> Self::State {
        0
    }
}

impl WidgetView for CounterWidget {
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
        let on_pressed = ctx.callback(|ctx, ()| {
            ctx.set_state(|state| {
                *state += 1;
            })
        });

        BuildResult::new(build! {
            Column {
                children: [
                    Text {
                        font: self.font.styled(),
                        text: format!("clicked: {} times", ctx.state).into()
                    },

                    ctx.key(
                        Key::single(),
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
                                    font: self.font.styled(),
                                    text: "A Button"
                                }
                            },
                            on_pressed
                        }
                    )
                ]
            }
        })
    }
}

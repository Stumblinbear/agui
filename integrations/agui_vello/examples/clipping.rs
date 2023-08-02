use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::prelude::*;
use agui_vello::{widgets::window::Window, AguiProgram};
use winit::{dpi::PhysicalSize, window::WindowBuilder};

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

    let deja_vu = Font::try_from_slice(include_bytes!("./fonts/DejaVuSans.ttf")).unwrap();

    let mut ui = AguiProgram::new();

    ui.set_root(
        Window::new(
            WindowBuilder::new()
                .with_title("agui clipping")
                .with_inner_size(PhysicalSize::new(800.0, 600.0)),
        )
        .with_child(ExampleMain { font: deja_vu }),
    );

    ui.run();
}

#[derive(StatelessWidget, PartialEq)]
struct ExampleMain {
    font: Font,
}

impl WidgetBuild for ExampleMain {
    type Child = Widget;

    fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
        build! {
            ColoredBox {
                color: Color::from_rgb((1.0, 1.0, 1.0)),

                child: Center {
                    child: SizedBox {
                        height: 16.0,
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
        }
    }
}

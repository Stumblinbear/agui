use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{app::run_app, prelude::*, winit::Window};
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

    // let mut renderer = VelloRenderer::new().expect("failed to init renderer");

    // let deja_vu = renderer.add_font(
    //     FontRef::new(include_bytes!("./fonts/DejaVuSans.ttf")).expect("failed to load font"),
    // );

    run_app(build! {
        <Window> {
            window: || WindowBuilder::new()
                .with_title("agui clipping")
                .with_inner_size(PhysicalSize::new(800.0, 600.0)),

            child: <ExampleMain>::default(),
        }
    });
}

#[derive(StatelessWidget, PartialEq, Default)]
struct ExampleMain;

impl StatelessWidget for ExampleMain {
    fn build(&self, _: &mut StatelessBuildContext<Self>) -> Widget {
        build! {
            <ColoredBox> {
                color: Color::from_rgb((1.0, 1.0, 1.0)),

                child: <Center> {
                    child: <SizedBox> {
                        height: Some(16.0),
                        width: Some(280.0),

                        child: <Clip> {
                            shape: Shape::RoundedRect {
                                top_left: 8.0,
                                top_right: 8.0,
                                bottom_right: 8.0,
                                bottom_left: 8.0,
                            },

                            child: <ColoredBox> {
                                color: Color::from_rgb((0.75, 0.75, 0.75)),

                                child: <Text> {
                                    style: TextStyle::default().color(Color::from_rgb((1.0, 0.0, 0.0))),
                                    text: "The Krabby Patty secret formula is one part love, two parts magic, and three parts secret ingredient.".into()
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

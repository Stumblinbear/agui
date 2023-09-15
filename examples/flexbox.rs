use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    prelude::*,
    vello::VelloRenderer,
    winit::{window::Window, App},
};
use vello::fello::raw::FontRef;
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

    App::with_renderer(
        VelloRenderer::new()
            .expect("failed to init renderer")
            .with_fonts([FontRef::new(include_bytes!("./fonts/DejaVuSans.ttf"))
                .expect("failed to load font")]),
    )
    .run(build! {
        <Window> {
            window: WindowBuilder::new()
                .with_title("agui flexbox")
                .with_inner_size(PhysicalSize::new(800.0, 600.0)),

            child: <Column> {
                main_axis_size: MainAxisSize::Min,
                main_axis_alignment: MainAxisAlignment::Start,

                children: [
                    <ColoredBox> {
                        color: Color::from_rgb((1.0, 0.0, 0.0)),

                        child: SizedBox::new(10.0, 10.0).into_widget(),
                    },
                    <Flexible> {
                        flex: Some(2.0),

                        child: <ColoredBox> {
                            color: Color::from_rgb((0.0, 1.0, 0.0)),

                            child: SizedBox::new(20.0, 10.0).into_widget(),
                        },
                    },
                    <ColoredBox> {
                        color: Color::from_rgb((0.0, 0.0, 1.0)),

                        child: SizedBox::new(30.0, 10.0).into_widget(),
                    },
                    <Flexible> {
                        flex: Some(1.0),

                        child: <ColoredBox> {
                            color: Color::from_rgb((1.0, 1.0, 0.0)),

                            child: SizedBox::new(30.0, 10.0).into_widget(),
                        },
                    },
                ]
            },
        }
    });
}

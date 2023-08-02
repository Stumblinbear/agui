use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{prelude::*, widgets::primitives::text::Text};
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

    ui.set_root(Stack {
        children: vec![
            Window {
                window: WindowBuilder::new()
                    .with_title("agui hello world")
                    .with_inner_size(PhysicalSize::new(800.0, 600.0)),

                child: Text::new("Hello, world!")
                    .with_font(deja_vu.styled().color(Color::from_rgb((1.0, 1.0, 1.0))))
                    .into_child(),
            }
            .into(),
            Window {
                window: WindowBuilder::new()
                    .with_title("agui goodbye world")
                    .with_inner_size(PhysicalSize::new(400.0, 300.0)),

                child: Text::new("Goodbye, world!")
                    .with_font(deja_vu.styled().color(Color::from_rgb((1.0, 1.0, 1.0))))
                    .into_child(),
            }
            .into(),
        ],
    });

    ui.run()
}

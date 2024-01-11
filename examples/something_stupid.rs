use agui_vello::{
    renderer::{window::VelloWindowRenderer, VelloRenderer},
    view::VelloView,
};
use agui_winit::WinitWindow;
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{app::run_app, prelude::*};
use winit::window::WindowBuilder;

fn main() {
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

    let vello_renderer = VelloRenderer::default();

    run_app(move || {
        let vello_renderer = vello_renderer.clone();

        let view_binding = vello_renderer.new_view_binding();

        build! {
            <WinitWindow> {
                window: || WindowBuilder::new().with_title("Hello, world!"),

                renderer: {
                    let view_binding = view_binding.clone();

                    move |window_handle| VelloWindowRenderer::new(view_binding.clone(), window_handle).expect("failed to create window renderer")
                },

                child: <VelloView> {
                    binding: view_binding,

                    child: <SizedBox>::axis(Axis::Horizontal, 100.0) {
                        child: <Text> {
                            style: TextStyle::default().color(Color::from_rgb((1.0, 1.0, 1.0))),
                            text: "Hello, world!".into(),
                        },
                    }
                }
            }
        }
    })
    .expect("Failed to run app");
}

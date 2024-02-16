use agui_vello::create_view::CreateVelloView;
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    app::run_app,
    prelude::*,
    vello::renderer::{window::VelloWindowRenderer, VelloRenderer},
    winit::{WinitWindow, WinitWindowAttributes},
};

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

    run_app(move || {
        let vello_renderer = VelloRenderer::default();

        build! {
            <Stack> {
                children: [
                    <CreateVelloView> {
                        renderer: vello_renderer.clone(),

                        builder: |view_handle| <WinitWindow> {
                            attributes: WinitWindowAttributes::builder()
                                .title("agui window 1")
                                .inner_size(Size::new(800.0, 600.0))
                                .build(),

                            renderer: VelloWindowRenderer::new(view_handle),

                            child: <Text> {
                                style: TextStyle::default().color(Color::from_rgb((1.0, 1.0, 1.0))),
                                text: "Hello, world!".into(),
                            }
                        }
                    },

                    <CreateVelloView> {
                        renderer: vello_renderer,

                        builder: |view_handle| <WinitWindow> {
                            exit_on_close: false,

                            attributes: WinitWindowAttributes::builder()
                                .title("agui window 2")
                                .inner_size(Size::new(400.0, 300.0))
                                .build(),

                            renderer: VelloWindowRenderer::new(view_handle),

                            child: <Text> {
                                style: TextStyle::default().color(Color::from_rgb((1.0, 1.0, 1.0))),
                                text: "Goodbye, world!".into(),
                            }
                        }
                    },
                ],
            }
        }
    })
    .expect("Failed to run app");
}

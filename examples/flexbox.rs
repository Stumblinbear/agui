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
            <CreateVelloView> {
                renderer: vello_renderer,

                builder: |view_handle| <WinitWindow> {
                    attributes: WinitWindowAttributes::builder()
                        .title("agui flexbox")
                        .build(),

                    renderer: VelloWindowRenderer::new(view_handle),

                    child: <Column> {
                        main_axis_size: MainAxisSize::Min,
                        main_axis_alignment: MainAxisAlignment::Start,

                        children: [
                            <ColoredBox> {
                                color: Color::from_rgb((1.0, 0.0, 0.0)),

                                child: <SizedBox>::new(10.0, 10.0),
                            },
                            <Flexible> {
                                flex: Some(2.0),

                                child: <ColoredBox> {
                                    color: Color::from_rgb((0.0, 1.0, 0.0)),

                                    child: <SizedBox>::new(20.0, 10.0),
                                },
                            },
                            <ColoredBox> {
                                color: Color::from_rgb((0.0, 0.0, 1.0)),

                                child: <SizedBox>::new(30.0, 10.0),
                            },
                            <Flexible> {
                                flex: Some(1.0),

                                child: <ColoredBox> {
                                    color: Color::from_rgb((1.0, 1.0, 0.0)),

                                    child: <SizedBox>::new(30.0, 10.0),
                                },
                            },
                        ]
                    }
                }
            }
        }
    })
    .expect("Failed to run app");
}

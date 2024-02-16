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
                        .title("Responsive widgets")
                        .build(),

                    renderer: VelloWindowRenderer::new(view_handle),

                    child:  <LayoutBuilder> {
                        resolver: |constraints| constraints.min_width() > 500.0,
                        builder: |is_larger| {
                            if *is_larger {
                                <ColoredBox> {
                                    color: Color::from_rgb((0.0, 1.0, 0.0)),

                                    child: <Text> {
                                        style: TextStyle::default().color(Color::from_rgb((1.0, 1.0, 1.0))),
                                        text: "Hello, world!".into(),
                                    },
                                }
                            }else{
                                <ColoredBox> {
                                    color: Color::from_rgb((0.0, 0.0, 1.0)),

                                    child: <Text> {
                                        style: TextStyle::default().color(Color::from_rgb((1.0, 1.0, 1.0))),
                                        text: "Hello, world!".into(),
                                    },
                                }
                            }
                        },
                    }
                }
            }
        }
    })
    .expect("Failed to run app");
}

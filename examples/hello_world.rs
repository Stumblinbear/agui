use agui_vello::create_view::CreateVelloView;
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    app::run_app,
    prelude::*,
    vello::renderer::{window::VelloWindowRenderer, VelloRenderer},
    winit::{WinitWindow, WinitWindowAttributes},
};

const DEJA_VU_FONT: &[u8] = include_bytes!("./fonts/DejaVuSans.ttf");

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
                renderer: vello_renderer.clone(),

                builder: |view_handle| <WinitWindow> {
                    attributes: WinitWindowAttributes::builder()
                        .title("agui hello world")
                        .build(),

                    renderer: VelloWindowRenderer::new(view_handle),

                    child: <Text> {
                        style: TextStyle {
                            font: Font::from_bytes(DEJA_VU_FONT.to_vec()),

                            size: 16.0,
                            color: Color::from_rgb((1.0, 1.0, 1.0)),

                            h_align: HorizontalAlign::default(),
                            v_align: VerticalAlign::default(),
                        },

                        text: "Hello, world!".into(),
                    }
                }
            }
        }
    })
    .expect("Failed to run app");
}

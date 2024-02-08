use agui_primitives::layout_builder::LayoutBuilder;
use agui_vello::{
    binding::VelloViewBinding,
    renderer::{window::VelloWindowRenderer, VelloRenderer},
};
use agui_winit::{WinitWindow, WinitWindowAttributes};
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{app::run_app, prelude::*};

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

    let vello_renderer = VelloRenderer::default();

    run_app(move || {
        let (view, view_handle) = vello_renderer.new_view();

        build! {
            <WinitWindow> {
                attributes: WinitWindowAttributes::builder()
                    .title("Responsive widgets")
                    .build(),

                renderer: VelloWindowRenderer::new(view_handle),

                child: <VelloViewBinding> {
                    view: view,

                    child: <LayoutBuilder> {
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

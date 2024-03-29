use std::time::Duration;

use agui_vello::create_view::CreateVelloView;
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    app::run_app,
    prelude::*,
    task::{context::ContextSpawnElementTask, TaskHandle},
    vello::renderer::{window::VelloWindowRenderer, VelloRenderer},
    winit::{CurrentWindow, WinitWindow, WinitWindowAttributes},
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
                renderer: vello_renderer,

                builder: |view_handle| <WinitWindow> {
                    attributes: WinitWindowAttributes::builder()
                        .title("agui current window")
                        .inner_size(Size::new(800.0, 600.0))
                        .build(),

                    renderer: VelloWindowRenderer::new(view_handle),

                    child: <ExampleMain>::default()
                }
            }
        }
    })
    .expect("Failed to run app");
}

#[derive(StatefulWidget, PartialEq, Default)]
struct ExampleMain;

impl StatefulWidget for ExampleMain {
    type State = ExampleMainState;

    fn create_state(&self) -> Self::State {
        ExampleMainState::default()
    }
}

#[derive(Default)]
struct ExampleMainState {
    update_count: usize,

    handle: Option<TaskHandle<()>>,
}

impl WidgetState for ExampleMainState {
    type Widget = ExampleMain;

    fn init_state(&mut self, ctx: &mut StatefulBuildContext<Self>) {
        let callback = ctx.callback(|ctx, _: ()| {
            ctx.set_state(move |state| {
                state.update_count += 1;
            });
        });

        self.handle = ctx
            .spawn_task(|_| async move {
                loop {
                    futures_timer::Delay::new(Duration::from_millis(1000)).await;

                    callback.call(());
                }
            })
            .ok();
    }

    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Widget {
        if let Some(current_window) = ctx.depend_on_inherited_widget::<CurrentWindow>() {
            current_window.set_title(&format!("agui current window - {}", self.update_count));
        } else {
            tracing::error!("CurrentWindow not found in the widget tree");
        }

        build! {
            <Text> {
                style: TextStyle {
                    font: Font::from_bytes(DEJA_VU_FONT.to_vec()),

                    size: 16.0,
                    color: Color::from_rgb((1.0, 1.0, 1.0)),

                    h_align: HorizontalAlign::default(),
                    v_align: VerticalAlign::default(),
                },

                text: format!("updated {} times", self.update_count).into(),
            }
        }
    }
}

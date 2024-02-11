use std::time::Duration;

use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    app::run_app,
    prelude::*,
    task::{context::ContextSpawnElementTask, TaskHandle},
    vello::{
        binding::VelloViewBinding,
        renderer::{window::VelloWindowRenderer, VelloRenderer},
    },
    winit::{CurrentWindow, WinitWindow, WinitWindowAttributes},
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

        let (view, view_handle) = vello_renderer.new_view();

        build! {
            <WinitWindow> {
                attributes: WinitWindowAttributes::builder()
                    .title("agui hello world")
                    .inner_size(Size::new(800.0, 600.0))
                    .build(),

                renderer: VelloWindowRenderer::new(view_handle),

                child: <VelloViewBinding> {
                    view: view,

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
            current_window.set_title(&format!("agui hello world - {}", self.update_count));
        } else {
            tracing::error!("CurrentWindow not found in the widget tree");
        }

        build! {
            <Text> {
                style: TextStyle::default().color(Color::from_rgb((1.0, 1.0, 1.0))),
                text: format!("updated {} times", self.update_count).into(),
            }
        }
    }
}

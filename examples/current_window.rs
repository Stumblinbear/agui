use std::{thread, time::Duration};

use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    app::run_app,
    prelude::*,
    winit::{CurrentWindow, Window},
};
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

    run_app(build! {
        <Window> {
            window: WindowBuilder::new()
                    .with_title("agui hello world")
                    .with_inner_size(PhysicalSize::new(800.0, 600.0)),

            child: <ExampleMain>::default(),
        }
    });
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
}

impl WidgetState for ExampleMainState {
    type Widget = ExampleMain;

    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Widget {
        let callback = ctx.callback::<usize, _>(|ctx, update_count| {
            ctx.set_state(move |state| {
                state.update_count = update_count;
            });
        });

        let update_count = self.update_count;

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1000));

            callback.call(update_count + 1);
        });

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

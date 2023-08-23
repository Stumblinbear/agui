use std::{thread, time::Duration};

use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    prelude::*,
    vello::VelloRenderer,
    winit::{window::Window, App},
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

    App::with_renderer(VelloRenderer::new().expect("failed to init renderer")).run(ExampleMain {
        window1: ColoredBox::new(Color::from_rgb((1.0, 0.0, 0.0)))
            .with_child(
                Padding::new(EdgeInsets::all(64.0))
                    .with_child(ColoredBox::new(Color::from_rgb((0.0, 1.0, 0.0)))),
            )
            .into(),
        window2: ColoredBox::new(Color::from_rgb((0.0, 0.0, 1.0)))
            .with_child(
                Padding::new(EdgeInsets::all(32.0))
                    .with_child(ColoredBox::new(Color::from_rgb((0.0, 1.0, 0.0)))),
            )
            .into(),
    });
}

#[derive(StatefulWidget)]
struct ExampleMain {
    window1: Widget,
    window2: Widget,
}

impl StatefulWidget for ExampleMain {
    type State = ExampleMainState;

    fn create_state(&self) -> Self::State {
        ExampleMainState {
            flip_windows: false,
        }
    }
}

struct ExampleMainState {
    flip_windows: bool,
}

impl WidgetState for ExampleMainState {
    type Widget = ExampleMain;

    type Child = Stack;

    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Self::Child {
        let callback = ctx.callback::<(), _>(|ctx, _| {
            ctx.set_state(|state| {
                state.flip_windows = !state.flip_windows;
            });
        });

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1000));

            callback.call(());
        });

        Stack::new()
            .add_child(
                Window::new(
                    WindowBuilder::new()
                        .with_title("agui window 1")
                        .with_inner_size(PhysicalSize::new(800.0, 600.0)),
                )
                .with_child(if self.flip_windows {
                    &ctx.widget.window2
                } else {
                    &ctx.widget.window1
                }),
            )
            .add_child(
                Window::new(
                    WindowBuilder::new()
                        .with_title("agui window 2")
                        .with_inner_size(PhysicalSize::new(400.0, 300.0)),
                )
                .with_child(if self.flip_windows {
                    &ctx.widget.window1
                } else {
                    &ctx.widget.window2
                }),
            )
    }
}

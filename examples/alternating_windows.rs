use std::{thread, time::Duration};

use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{app::run_app, prelude::*, winit::Window};
use winit::{dpi::PhysicalSize, window::WindowBuilder};

fn main() {
    let filter = EnvFilter::from_default_env()
        .add_directive(LevelFilter::ERROR.into())
        .add_directive(
            format!("agui={}", LevelFilter::INFO)
                .parse()
                .expect("Failed to parse log level directive"),
        );

    tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::time())
        .with_level(true)
        .with_thread_names(false)
        .with_target(true)
        .with_env_filter(filter)
        .init();

    run_app(build! {
        <ExampleMain> {
            window1: <ColoredBox> {
                color: Color::from_rgb((1.0, 0.0, 0.0)),

                child: <Padding> {
                    padding: EdgeInsets::all(64.0),

                    child: <ColoredBox> {
                        color: Color::from_rgb((0.0, 1.0, 0.0)),
                    },
                },
            },
            window2: <ColoredBox> {
                color: Color::from_rgb((0.0, 0.0, 1.0)),

                child: <Padding> {
                    padding: EdgeInsets::all(32.0),

                    child: <ColoredBox> {
                        color: Color::from_rgb((0.0, 1.0, 0.0)),
                    },
                },
            },
        }
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

    fn init_state(&mut self, ctx: &mut StatefulBuildContext<Self>) {
        let callback = ctx.callback::<(), _>(|ctx, _| {
            ctx.set_state(|state| {
                state.flip_windows = !state.flip_windows;
            });
        });

        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(1000));

            callback.call(());
        });
    }

    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Widget {
        build! {
            <Stack> {
                children: [
                    <Window> {
                        window: || WindowBuilder::new()
                            .with_title("agui window 1")
                            .with_inner_size(PhysicalSize::new(800.0, 600.0)),

                        child: if self.flip_windows {
                            ctx.widget.window2.clone()
                        } else {
                            ctx.widget.window1.clone()
                        }
                    },
                    <Window> {
                        window: || WindowBuilder::new()
                            .with_title("agui window 2")
                            .with_inner_size(PhysicalSize::new(400.0, 300.0)),

                        child: if self.flip_windows {
                            ctx.widget.window1.clone()
                        } else {
                            ctx.widget.window2.clone()
                        }
                    },
                ]
            }
        }
    }
}

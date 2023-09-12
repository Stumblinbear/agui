use std::{thread, time::Duration};

use agui_winit::window::CurrentWindow;
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    prelude::*,
    vello::VelloRenderer,
    winit::{window::Window, App},
};
use vello::fello::raw::FontRef;
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

    App::with_renderer(
        VelloRenderer::new()
            .expect("failed to init renderer")
            .with_fonts([FontRef::new(include_bytes!("./fonts/DejaVuSans.ttf"))
                .expect("failed to load font")]),
    )
    .run(build! {
        <Window> {
            window: WindowBuilder::new()
                    .with_title("agui hello world")
                    .with_inner_size(PhysicalSize::new(800.0, 600.0)),

            child: <ExampleMain> {
                font: Font::default(),
            },
        }
    });
}

#[derive(StatefulWidget, PartialEq, Default)]
struct ExampleMain {
    font: Font,
}

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
                font: Font::default()
                    .styled()
                    .color(Color::from_rgb((1.0, 1.0, 1.0))),

                text: format!("updated {} times", self.update_count).into(),
            }
        }
    }
}

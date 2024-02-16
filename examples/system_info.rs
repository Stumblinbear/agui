use std::time::Duration;

use agui_vello::create_view::CreateVelloView;
use sysinfo::{System, SystemExt};
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    app::run_app,
    prelude::*,
    task::{context::ContextSpawnElementTask, TaskHandle},
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
                        .title("agui hello world")
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

#[derive(Clone, Debug)]
pub struct SystemInfo {
    total_memory: u64,
    used_memory: u64,

    total_swap: u64,
    used_swap: u64,

    name: String,
    kernel_version: String,
    os_version: String,
    host_name: String,

    processors: usize,
}

#[derive(Default, StatefulWidget, PartialEq)]
struct ExampleMain;

impl StatefulWidget for ExampleMain {
    type State = ExampleMainState;

    fn create_state(&self) -> Self::State {
        ExampleMainState::default()
    }
}

#[derive(Default)]
struct ExampleMainState {
    system_info: Option<SystemInfo>,

    handle: Option<TaskHandle<()>>,
}

impl WidgetState for ExampleMainState {
    type Widget = ExampleMain;

    fn init_state(&mut self, ctx: &mut StatefulBuildContext<Self>) {
        let callback = ctx.callback::<SystemInfo, _>(|ctx, system_info| {
            ctx.set_state(|state| {
                state.system_info.replace(system_info);
            });
        });

        self.handle = ctx
            .spawn_task(move |_| async move {
                loop {
                    futures_timer::Delay::new(Duration::from_millis(1000)).await;

                    let mut system = System::new_all();

                    system.refresh_all();

                    callback.call(SystemInfo {
                        total_memory: system.total_memory(),
                        used_memory: system.used_memory(),

                        total_swap: system.total_swap(),
                        used_swap: system.used_swap(),

                        name: system.name().unwrap_or_else(|| "---".into()),
                        kernel_version: system.kernel_version().unwrap_or_else(|| "---".into()),
                        os_version: system.os_version().unwrap_or_else(|| "---".into()),
                        host_name: system.host_name().unwrap_or_else(|| "---".into()),

                        processors: system.cpus().len(),
                    });
                }
            })
            .ok();
    }

    fn build(&mut self, _: &mut StatefulBuildContext<Self>) -> Widget {
        let lines = match &self.system_info {
            None => vec!["Collecting system info...".into()],

            Some(sys) => vec![
                format!("System name: {}", sys.name),
                format!("System kernel version: {}", sys.kernel_version),
                format!("System OS version: {}", sys.os_version),
                format!("System host name: {}", sys.host_name),
                "".into(),
                format!("NB processors: {}", sys.processors),
                "".into(),
                format!("Total Memory: {} KB", sys.total_memory),
                format!("Used Memory: {} KB", sys.used_memory),
                format!("Total Swap: {} KB", sys.total_swap),
                format!("Used Swap: {} KB", sys.used_swap),
            ],
        };

        build! {
            <Center> {
                child: <ColoredBox> {
                    color: Color::from_rgb((1.0, 1.0, 1.0)),

                    child: <Column> {
                        main_axis_size: MainAxisSize::Min,
                        main_axis_alignment: MainAxisAlignment::Center,

                        children: lines
                            .into_iter()
                            .map(|entry| <Text> {
                                style: TextStyle::default().color(Color::from_rgb((0.0, 0.0, 0.0))),
                                text: entry.into(),
                            })
                            .collect::<Vec<_>>()
                    }
                }
            }
        }
    }
}

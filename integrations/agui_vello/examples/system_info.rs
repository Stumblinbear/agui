#![allow(clippy::needless_update)]
use std::{thread, time::Duration};

use sysinfo::{System, SystemExt};
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    prelude::*,
    widgets::{
        primitives::{Center, ColoredBox, Column, MainAxisAlignment, MainAxisSize, Text},
        App,
    },
};
use agui_vello::AguiProgram;

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

    let mut ui = AguiProgram::new(
        "agui os info",
        Size {
            width: 800.0,
            height: 600.0,
        },
    );

    let deja_vu = ui
        .load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"))
        .unwrap();

    ui.set_root(App {
        child: ExampleMain { font: deja_vu }.into(),
    });

    ui.run()
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

#[derive(StatefulWidget, PartialEq)]
struct ExampleMain {
    font: Font,
}

impl WidgetState for ExampleMain {
    type State = Option<SystemInfo>;

    fn create_state(&self) -> Self::State {
        None
    }
}

impl WidgetView for ExampleMain {
    fn build(&self, ctx: &mut BuildContext<Self>) -> Children {
        let callback = ctx.callback::<SystemInfo, _>(|ctx, system_info| {
            ctx.set_state(|state| {
                state.replace(system_info.clone());
            });
        });

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1000));

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
        });

        let lines = match ctx.get_state() {
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

        Children::new(build! {
            Center {
                child: ColoredBox {
                    color: Color::from_rgb((1.0, 1.0, 1.0)),

                    child: Column {
                        main_axis_size: MainAxisSize::Min,

                        main_axis_alignment: MainAxisAlignment::Center,

                        children: lines
                            .into_iter()
                            .map(|entry| {
                                Text {
                                    font: self.font.styled().color(Color::from_rgb((0.0, 0.0, 0.0))),
                                    text: entry.into(),
                                }
                                .into()
                            })
                            .collect::<Vec<_>>(),
                    }
                }
            }
        })
    }
}

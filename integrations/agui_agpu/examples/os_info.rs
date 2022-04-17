#![allow(clippy::needless_update)]
use std::{thread, time::Duration};

use sysinfo::{System, SystemExt};
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

use agui::{
    macros::{build, functional_widget},
    prelude::*,
    widgets::{
        plugins::DefaultPluginsExt,
        primitives::{Column, Spacing, Text},
        App,
    },
};
use agui_agpu::UIProgram;

fn main() -> Result<(), agpu::BoxError> {
    let filter = EnvFilter::from_default_env()
        .add_directive(LevelFilter::ERROR.into())
        .add_directive(format!("agui={}", LevelFilter::DEBUG).parse().unwrap());

    tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::time())
        .with_level(true)
        .with_thread_names(false)
        .with_target(true)
        .with_env_filter(filter)
        .init();

    let mut ui = UIProgram::new("agui os info")?;

    ui.register_default_plugins();
    // ui.register_default_globals();

    let deja_vu = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"))?;

    ui.set_root(build! {
        App {
            child: ExampleMain {
                font: deja_vu
            }
        }
    });

    ui.run()
}

#[derive(Debug)]
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

type OptSystem = Option<SystemInfo>;

#[functional_widget(OptSystem)]
fn example_main(ctx: &mut BuildContext, font: Font, _color: Color, _child: Widget) -> BuildResult {
    ctx.set_layout(Layout {
        sizing: Sizing::Fill,
        ..Layout::default()
    });

    let callback = ctx.callback::<System, _>(|ctx, system| {
        ctx.set_state(|state| {
            state.replace(SystemInfo {
                total_memory: system.total_memory(),
                used_memory: system.used_memory(),

                total_swap: system.total_swap(),
                used_swap: system.used_swap(),

                name: system.name().unwrap_or_else(|| "---".into()),
                kernel_version: system.kernel_version().unwrap_or_else(|| "---".into()),
                os_version: system.os_version().unwrap_or_else(|| "---".into()),
                host_name: system.host_name().unwrap_or_else(|| "---".into()),

                processors: system.processors().len(),
            });
        });
    });

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(1000));

        let mut system = System::new_all();

        system.refresh_all();

        callback.call(system);
    });

    let children: Vec<Widget> = match ctx.state {
        None => vec![Text {
            font: font.styled().color(Color::White),
            text: "Collecting system info...".into(),
        }
        .into()],

        Some(sys) => vec![
            Text {
                font: font.styled().color(Color::White),
                text: format!("System name: {}", sys.name).into(),
            }
            .into(),
            Text {
                font: font.styled().color(Color::White),
                text: format!("System kernel version: {}", sys.kernel_version).into(),
            }
            .into(),
            Text {
                font: font.styled().color(Color::White),
                text: format!("System OS version: {}", sys.os_version).into(),
            }
            .into(),
            Text {
                font: font.styled().color(Color::White),
                text: format!("System host name: {}", sys.host_name).into(),
            }
            .into(),
            Spacing::vertical(16.0.into()).into(),
            Text {
                font: font.styled().color(Color::White),
                text: format!("NB processors: {}", sys.processors).into(),
            }
            .into(),
            Spacing::vertical(16.0.into()).into(),
            Text {
                font: font.styled().color(Color::White),
                text: format!("Total Memory: {} KB", sys.total_memory).into(),
            }
            .into(),
            Text {
                font: font.styled().color(Color::White),
                text: format!("Used Memory: {} KB", sys.used_memory).into(),
            }
            .into(),
            Text {
                font: font.styled().color(Color::White),
                text: format!("Total Swap: {} KB", sys.total_swap).into(),
            }
            .into(),
            Text {
                font: font.styled().color(Color::White),
                text: format!("Used Swap: {} KB", sys.used_swap).into(),
            }
            .into(),
        ],
    };

    build! {
        Column {
            layout: Layout {
                sizing: Sizing::Axis {
                    width: Units::Stretch(1.0),
                    height: Units::Auto
                },
            },
            children
        }
    }
}

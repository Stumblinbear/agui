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
}

type OptSystem = Option<SystemInfo>;

#[functional_widget(OptSystem)]
fn example_main(ctx: &mut BuildContext, font: Font, _color: Color, _child: Widget) -> BuildResult {
    ctx.set_layout(Layout {
        sizing: Sizing::Fill,
        ..Layout::default()
    });

    if ctx.state.is_none() {
        let callback = ctx.callback::<System, _>(|ctx, system| {
            ctx.set_state(|state| {
                state.replace(SystemInfo {
                    total_memory: system.total_memory(),
                    used_memory: system.used_memory(),

                    total_swap: system.total_swap(),
                    used_swap: system.used_swap(),
                });
            });
        });

        thread::spawn(move || {
            println!("Delaying...");

            thread::sleep(Duration::from_millis(2000));

            println!("Collecting system information...");

            let mut system = System::new_all();

            system.refresh_all();

            println!("Emitting callback");

            callback.call(system);
        });
    }

    let mut children: Vec<Widget> = vec![
        Text {
            font: font.styled().size(38.0).color(Color::White),
            text: "System Info".into(),
        }
        .into(),
        Spacing::vertical(16.0.into()).into(),
    ];

    match ctx.state {
        None => children.push(
            Text {
                font: font.styled().color(Color::White),
                text: "Collecting system info...".into(),
            }
            .into(),
        ),

        Some(sys) => children.extend(vec![
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
        ]),
    }

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

#![allow(clippy::needless_update)]

use agpu::Features;
use agui::{
    layout::Layout,
    macros::build,
    unit::{Callback, Sizing, Units},
    widgets::{
        plugins::hovering::HoveringPlugin,
        primitives::{Column, Text, Quad},
        state::{
            hovering::Hovering,
            keyboard::{Keyboard, KeyboardInput},
            mouse::{Mouse, Scroll},
        },
        App, Button,
    },
};
use agui_agpu::UI;

fn main() -> Result<(), agpu::BoxError> {
    let program = agpu::GpuProgram::builder("agui widgets")
        .with_gpu_features(Features::POLYGON_MODE_LINE)
        .with_framerate(60.0)
        .build()?;

    let mut ui = UI::with_default(&program);

    ui.init_plugin::<HoveringPlugin>();

    ui.get_context().init_global::<Keyboard>();
    ui.get_context().init_global::<KeyboardInput>();

    ui.get_context().init_global::<Mouse>();
    ui.get_context().init_global::<Scroll>();

    ui.get_context().init_global::<Hovering>();

    ui.set_root(build! {
        App {
            child: Column {
                children: [
                    Button {
                        layout: Layout {
                            sizing: Sizing::Set {
                                width: 100,
                                height: 100
                            }
                        },
                        child: Text {
                            text: "A Button"
                        },
                        on_pressed: Callback::from(|()| {
                            println!("Pressed");
                        })
                    },
                    Button {
                        layout: Layout {
                            sizing: Sizing::Set {
                                width: 100,
                                height: 100
                            }
                        },
                        child: Text {
                            text: "A Button"
                        },
                        on_pressed: Callback::from(|()| { })
                    },
                    Button {
                        layout: Layout {
                            sizing: Sizing::Set {
                                width: 100,
                                height: 100
                            }
                        },
                        child: Text {
                            text: "A Button"
                        },
                        on_pressed: Callback::from(|()| { })
                    }
                ]
            }
        }
    });

    ui.run(program)
}

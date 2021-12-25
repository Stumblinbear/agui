#![allow(clippy::needless_update)]

use agpu::Features;
use agui::{
    layout::Layout,
    macros::build,
    unit::{Sizing, Units},
    widgets::{
        primitives::{Column, Text},
        state::{
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
        .build()?;

    let mut ui = UI::with_default(&program);

    ui.get_context().init_global::<Keyboard>();
    ui.get_context().init_global::<KeyboardInput>();

    ui.get_context().init_global::<Mouse>();
    ui.get_context().init_global::<Scroll>();

    ui.set_root(build! {
        App {
            child: Column {
                children: vec! [
                    Button {
                        layout: Layout {
                            sizing: Sizing::Set {
                                width: Units::Pixels(100.0),
                                height: Units::Pixels(100.0)
                            }
                        },
                        child: Text {
                            text: "A Button".into()
                        }
                    },
                    Button {
                        layout: Layout {
                            sizing: Sizing::Set {
                                width: Units::Pixels(100.0),
                                height: Units::Pixels(100.0)
                            }
                        },
                        child: Text {
                            text: "A Button".into()
                        }
                    },
                    Button {
                        layout: Layout {
                            sizing: Sizing::Set {
                                width: Units::Pixels(100.0),
                                height: Units::Pixels(100.0)
                            }
                        },
                        child: Text {
                            text: "A Button".into()
                        }
                    }
                ].into()
            }
        }
    });

    ui.run(program)
}

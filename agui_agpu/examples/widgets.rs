#![allow(clippy::needless_update)]

use agpu::{Event, Features};
use agui::{
    layout::Layout,
    macros::build,
    unit::{Sizing, Units},
    widgets::{
        primitives::{Column, Text},
        App, Button,
    },
};
use agui_agpu::UI;

fn main() -> Result<(), agpu::BoxError> {
    let program = agpu::GpuProgram::builder("agui widgets")
        .with_gpu_features(Features::POLYGON_MODE_LINE)
        .build()?;

    let mut ui = UI::with_default(&program);

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

    let pipeline = program.gpu.new_pipeline("render pipeline").create();

    program.run(move |event, _, _| {
        if let Event::RedrawFrame(mut frame) = event {
            if ui.update() {
                // ui.get_manager().print_tree();
    
                frame
                    .render_pass_cleared("ui draw", 0x101010FF)
                    .with_pipeline(&pipeline)
                    .begin();

                ui.render(frame);
            }
        } else if let Event::Winit(event) = event {
        }
    });
}

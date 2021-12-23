#![allow(clippy::needless_update)]

use agui::{
    layout::Layout,
    macros::build,
    unit::{Sizing, Units},
    widgets::{
        primitives::{Column, Text},
        App, AppSettings, Button,
    },
    UI,
};

fn main() -> Result<(), agpu::BoxError> {
    let program = agpu::GpuProgram::builder("agui widgets").build()?;

    let mut ui = UI::new(agui_agpu::WidgetRenderer::new(&program));

    let settings = ui.get_context().init_global::<AppSettings>();

    program.on_resize(move |_, w, h| {
        let mut settings = settings.write();

        settings.width = w as f32;
        settings.height = h as f32;
    });

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

    let gpu = program.gpu.clone();

    let pipeline = gpu.new_pipeline("example render pipeline").create();

    program.run_draw(move |mut frame| {
        frame
            .render_pass_cleared("scene draw pass", 0x101010FF)
            .with_pipeline(&pipeline)
            .begin()
            .draw_triangle();

        if ui.update() {
            ui.get_manager().print_tree();
            
            ui.get_renderer().render(frame);
        }
    })
}

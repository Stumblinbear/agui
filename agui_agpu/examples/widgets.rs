#![allow(clippy::needless_update)]

use agpu::Features;
use agui::{
    context::Ref,
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
    let program = agpu::GpuProgram::builder("agui widgets")
        .with_gpu_features(Features::POLYGON_MODE_LINE)
        .build()?;

    let mut ui = UI::new(agui_agpu::WidgetRenderer::new(&program));

    let settings = ui.get_context().set_global(AppSettings {
        width: program.viewport.inner_size().width as f32,
        height: program.viewport.inner_size().height as f32,
    });

    ui.get_renderer_mut()
        .set_app_settings(Ref::clone(&settings));

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

    let pipeline = program.gpu.new_pipeline("render pipeline").create();

    program.run_draw(move |mut frame| {
        frame
            .render_pass_cleared("ui draw", 0x101010FF)
            .with_pipeline(&pipeline)
            .begin();

        if ui.update() {
            // ui.get_manager().print_tree();

            ui.get_renderer_mut().render(frame);
        }
    })
}

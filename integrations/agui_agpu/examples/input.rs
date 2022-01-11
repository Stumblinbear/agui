#![allow(clippy::needless_update)]

use agpu::Features;
use agui::{
    layout::Layout,
    macros::build,
    unit::{Sizing, Units},
    widgets::{
        plugins::{hovering::HoveringPlugin, timeout::TimeoutPlugin},
        App, TextInput,
    },
};
use agui_agpu::UI;

fn main() -> Result<(), agpu::BoxError> {
    let program = agpu::GpuProgram::builder("agui: input")
        .with_gpu_features(
            Features::POLYGON_MODE_LINE
                | Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                | Features::VERTEX_WRITABLE_STORAGE,
        )
        .build()?;

    let mut ui = UI::with_default(&program);

    ui.get_context().init_plugin(HoveringPlugin::default);
    ui.get_context().init_plugin(TimeoutPlugin::default);

    let deja_vu_sans = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"));

    ui.set_root(build! {
        App {
            child: TextInput {
                layout: Layout {
                    sizing: Sizing::Axis {
                        width: Units::Stretch(1.0),
                        height: Units::Auto,
                    }
                },

                font: deja_vu_sans,
                placeholder: "some text here"
            }
        }
    });

    ui.run(program)
}

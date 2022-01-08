#![allow(clippy::needless_update)]

use agpu::Features;
use agui::{
    macros::build,
    widgets::{primitives::Text, App},
};
use agui_agpu::UI;

fn main() -> Result<(), agpu::BoxError> {
    let program = agpu::GpuProgram::builder("agui: Hello, world!")
        .with_gpu_features(
            Features::POLYGON_MODE_LINE
                | Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                | Features::VERTEX_WRITABLE_STORAGE,
        )
        .build()?;

    let mut ui = UI::with_default(&program);

    let deja_vu_sans = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"));

    ui.set_root(build! {
        App {
            child: Text::is(deja_vu_sans, 32.0, "Hello, world!".into())
        }
    });

    ui.run(program)
}

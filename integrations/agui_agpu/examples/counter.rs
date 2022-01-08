#![allow(clippy::needless_update)]

use agui::{
    macros::{build, functional_widget},
    widgets::{primitives::{Text, Column, FontId, Padding}, App, Button, plugins::hovering::HoveringPlugin, state::mouse::{Mouse, Scroll}}, context::WidgetContext, widget::BuildResult, unit::{Margin, Callback},
};
use agui_agpu::UI;

fn main() -> Result<(), agpu::BoxError> {
    let program = agpu::GpuProgram::builder("agui: Hello, world!")
        .with_gpu_features(
            agpu::Features::POLYGON_MODE_LINE
                | agpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                | agpu::Features::VERTEX_WRITABLE_STORAGE,
        )
        .build()?;

    let mut ui = UI::with_default(&program);

    ui.get_context().init_plugin(HoveringPlugin::default);

    ui.get_context().init_global(Mouse::default);
    ui.get_context().init_global(Scroll::default);

    let deja_vu_sans = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"));

    ui.set_root(build! {
        App {
            child: CounterWidget {
                font: deja_vu_sans
            }
        }
    });

    ui.run(program)
}

#[functional_widget]
fn counter_widget(ctx: &WidgetContext, font: FontId) -> BuildResult {
    let num = ctx.use_state(|| 0);

    build! {
        Column {
            children: [
                Text::is(font, 32.0, format!("clicked: {} times", num.read())),
                Button {
                    child: Padding {
                        padding: Margin::All(10.0.into()),
                        child: Text::is(font, 32.0, "A Button".into())
                    },
                    on_pressed: Callback::from(move |()| {
                        *num.write() += 1;
                    })
                }
            ]
        }
    }
}
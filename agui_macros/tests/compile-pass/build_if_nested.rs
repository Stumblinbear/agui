use agui_core::{unit::Color, widget::WidgetRef};
use agui_macros::build;
use agui_primitives::Quad;

fn main() {
    let i = 0.0;

    let _widget: WidgetRef = build! {
        Quad {
            child: if i > 1.0 {
                Quad
            } else {
                Quad
            },
        }
    };
}

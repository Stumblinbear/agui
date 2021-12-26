use agui_core::{unit::Color, widget::WidgetRef};
use agui_macros::build;
use agui_primitives::Quad;

fn main() {
    let _widget: WidgetRef = build! {
        Quad {
            color: Color::Black,
        }
    };
}

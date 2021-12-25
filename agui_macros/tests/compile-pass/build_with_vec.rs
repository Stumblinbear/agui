use agui_core::{unit::Color, widget::WidgetRef};
use agui_macros::build;
use agui_primitives::Quad;

fn main() {
    let _widget: WidgetRef = build! {
        Column {
            // TODO: when [] is added, update this test
            children: vec! {
                Quad,
                Quad
            }
        }
    };
}

use agui_core::WidgetRef;
use agui_macros::build;
use agui_primitives::{Quad, Column};

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

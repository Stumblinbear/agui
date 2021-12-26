use agui_core::widget::WidgetRef;
use agui_macros::build;
use agui_primitives::{Column, Quad};

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

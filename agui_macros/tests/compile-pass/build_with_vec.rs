use agui_core::widget::WidgetRef;
use agui_macros::build;
use agui_primitives::{Column, Quad};

fn main() {
    let _widget: WidgetRef = build! {
        Column {
            children: [
                Quad { },
                Quad { }
            ]
        }
    };
}

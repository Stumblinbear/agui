use agui_core::widget::WidgetRef;
use agui_macros::build;
use agui_primitives::{Column, Row};

fn main() {
    let _widget: WidgetRef = build! {
        Column {
            children: [
                Row { },
                Row { }
            ]
        }
    };
}

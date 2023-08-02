use agui_core::widget::Widget;
use agui_macros::build;
use agui_primitives::{Column, Row};

fn main() {
    let _widget: Widget = build! {
        Column {
            children: [
                Row { },
                Row { }
            ]
        }
    };
}

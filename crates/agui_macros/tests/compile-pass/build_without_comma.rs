use agui_core::widget::Widget;
use agui_macros::build;
use agui_primitives::{Padding, Row};

fn main() {
    let _widget: Widget = build! {
        Padding {
            child: Row { }
        }
    };
}

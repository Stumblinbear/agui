use agui_core::widget::WidgetRef;
use agui_macros::build;
use agui_primitives::{Padding, Row};

fn main() {
    let _widget: WidgetRef = build! {
        Padding {
            child: Row { }
        }
    };
}

use agui_core::widget::WidgetRef;
use agui_macros::build;
use agui_primitives::{Column, Drawable};

fn main() {
    let _widget: WidgetRef = build! {
        Column {
            children: [
                Drawable { },
                Drawable { }
            ]
        }
    };
}

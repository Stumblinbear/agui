use agui_core::{unit::Margin, widget::Widget};
use agui_macros::build;
use agui_primitives::Padding;

fn main() {
    let _widget: Widget = build! {
        Padding {
            padding: Margin::Unset,
        }
    };
}

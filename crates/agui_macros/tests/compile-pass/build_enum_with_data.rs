use agui_core::{unit::Margin, widget::WidgetRef};
use agui_macros::build;
use agui_primitives::Padding;

fn main() {
    let _widget: WidgetRef = build! {
        Padding {
            padding: Margin::All(10.0.into()),
        }
    };
}

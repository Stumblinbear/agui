use agui_core::{unit::Margin, widget::Widget};
use agui_macros::build;
use agui_primitives::{Padding, Row};

fn main() {
    let _widget: Widget = build! {
        Padding {
            padding: Margin::All(10.0.into()),
            child: Row { }
        }
    };
}

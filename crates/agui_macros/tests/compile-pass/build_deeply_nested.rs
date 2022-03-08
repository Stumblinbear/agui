use agui_core::{unit::Margin, widget::WidgetRef};
use agui_macros::build;
use agui_primitives::{Column, Padding, Row};

fn main() {
    let _widget: WidgetRef = build! {
        Padding {
            padding: Margin::All(10.0.into()),
            child: Column {
                children: [
                    Row { },
                    Row { },
                ]
            }
        }
    };
}

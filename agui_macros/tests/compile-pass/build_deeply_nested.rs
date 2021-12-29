use agui_core::{unit::Color, widget::WidgetRef};
use agui_macros::build;
use agui_primitives::{Quad, QuadStyle};

fn main() {
    let _widget: WidgetRef = build! {
        Quad {
            style: QuadStyle {
                color: Color::Black,
            },
            child: Quad {
                style: QuadStyle {
                    color: Color::Rgb(1.0, 1.0, 1.0),
                },
                child: Quad {
                    style: QuadStyle {
                        color: Color::White,
                    },
                    child: Quad { }
                }
            }
        }
    };
}

use agui_core::{WidgetRef, render::color::Color};
use agui_macros::build;
use agui_primitives::Quad;

fn main() {
    let _widget: WidgetRef = build! {
        Quad {
            color: Color::Black,
            child: Quad {
                color: Color::Rgb(1.0, 1.0, 1.0),
                child: Quad {
                    color: Color::White,
                    child: Quad
                }
            }
        }
    };
}

use agui_core::{unit::Color, widget::WidgetRef};
use agui_macros::build;
use agui_primitives::{Drawable, DrawableStyle};

fn main() {
    let _widget: WidgetRef = build! {
        Drawable {
            style: DrawableStyle {
                color: Color::Black,
            },
            child: Drawable {
                style: DrawableStyle {
                    color: Color::Rgb(1.0, 1.0, 1.0),
                },
                child: Drawable {
                    style: DrawableStyle {
                        color: Color::White,
                    },
                    child: Drawable { }
                }
            }
        }
    };
}

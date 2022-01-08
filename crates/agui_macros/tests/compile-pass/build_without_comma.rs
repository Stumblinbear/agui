use agui_core::widget::WidgetRef;
use agui_macros::build;
use agui_primitives::Drawable;

fn main() {
    let _widget: WidgetRef = build! {
        Drawable {
            child: Drawable { }
        }
    };
}

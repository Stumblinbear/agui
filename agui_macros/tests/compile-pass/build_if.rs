use agui_core::widget::WidgetRef;
use agui_macros::build;
use agui_primitives::Quad;

fn main() {
    let i = 0.0;

    let _widget: WidgetRef = build! {
        if i > 1.0 {
            Quad
        }else{
            Quad
        }
    };
}

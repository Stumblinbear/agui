pub use agui_core::*;

#[cfg(any(feature = "primitives", feature = "widgets"))]
pub mod widgets {
    #[cfg(feature = "primitives")]
    pub mod primitives {
        pub use agui_primitives::*;
    }

    #[cfg(feature = "widgets")]
    pub use agui_widgets::*;
}

#[cfg(feature = "macros")]
pub mod macros {
    pub use agui_macros::*;
}

#[cfg(test)]
mod tests {
    use agui_core::WidgetRef;
    use agui_macros::build;

    use crate::{
        unit::{Layout, Sizing, Units},
        widgets::{primitives::{Quad, Text}, Button},
    };

    #[allow(clippy::needless_update)]
    #[warn(unused_variables)]
    #[test]
    pub fn test() {
        let no_block: WidgetRef = build! {
            Button
        };

        let with_block: WidgetRef = build! {
            Button { }
        };

        let without_field_comma: WidgetRef = build! {
            Button {
                layout: Layout
            }
        };

        let with_field_comma: WidgetRef = build! {
            Button {
                layout: Layout,
            }
        };

        let enum_usage: WidgetRef = build! {
            Quad {
                layout: Layout {
                    sizing: Sizing::Fill
                }
            }
        };

        let enum_with_data: WidgetRef = build! {
            Button {
                layout: Layout {
                    sizing: Sizing::Set {
                        width: Units::Pixels(100.0),
                        height: Units::Pixels(100.0),
                    }
                }
            }
        };

        let i = 0;

        let nested_if: WidgetRef = build! {
            Button {
                layout: Layout,
                child: if i < 20 {
                    Button
                }else{
                    Text
                }
            }
        };

        let widget: WidgetRef = build! {
            Quad {
                layout: Layout {
                    sizing: Sizing::Fill
                },
                child: Text {
                    text: String::from("")
                }
            }
        };
        
        let complex: WidgetRef = build! {
            if i > 10 {
                Button {
                    layout: Layout,
                    child: if i < 20 {
                        Button
                    }else{
                        Text
                    }
                }
            }else{
                Button
            }
        };
    }

    // #[test]
    // #[should_panic]
    // fn test_failures() {
    //     let fail: WidgetRef = build! { };

    //     let no_field: WidgetRef = build! {
    //         Quad {
    //             layout: 
    //         }
    //     };

    //     let bad_into: WidgetRef = build! {
    //         Layout {
    //             sizing: Sizing::Fill
    //         }
    //     };
    // }
}
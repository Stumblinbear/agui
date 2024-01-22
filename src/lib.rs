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

#[cfg(feature = "winit")]
pub use agui_winit as winit;

#[cfg(feature = "vello")]
pub use agui_vello as vello;

pub mod prelude {
    pub use agui_core::{callback::*, query::*, render::*, unit::*, widget::*};
    pub use agui_elements::{render::*, stateful::*, stateless::*};
    pub use agui_primitives::{
        align::{Align, Center},
        builder::Builder,
        clip::Clip,
        colored_box::ColoredBox,
        flex::{
            Column, CrossAxisAlignment, Flex, FlexFit, Flexible, MainAxisAlignment, MainAxisSize,
            Row, VerticalDirection,
        },
        intrinsic::{IntrinsicAxis, IntrinsicHeight, IntrinsicWidth},
        padding::Padding,
        sized_box::SizedBox,
        stack::Stack,
        text::{Text, TextBaseline},
    };

    #[cfg(feature = "macros")]
    pub use agui_macros::*;
}

#[cfg(feature = "app")]
pub mod app;

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
    pub use agui_primitives::{
        align::*, builder::*, clip::*, colored_box::*, flex::*, intrinsic::*, padding::*,
        sized_box::*, stack::*, text::*,
    };

    #[cfg(feature = "macros")]
    pub use agui_macros::*;
}

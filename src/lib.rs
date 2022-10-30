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

pub mod prelude {
    pub use agui_core::{callback::*, context::*, query::*, render::*, unit::*, widget::*};

    #[cfg(feature = "macros")]
    pub use agui_macros::*;
}

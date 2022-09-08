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

pub mod prelude {
    pub use agui_core::{
        callback::Callback,
        query::*,
        render::canvas::paint::{Brush, Paint},
        unit::*,
        widget::{BuildContext, BuildResult, Widget, WidgetBuilder, WidgetContext},
    };
}

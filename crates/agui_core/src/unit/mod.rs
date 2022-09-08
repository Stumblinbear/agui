pub(crate) const POS_MARGIN_OF_ERROR: f32 = 0.01;
pub(crate) const COLOR_MARGIN_OF_ERROR: f32 = 0.001;

mod blend_mode;
mod color;
mod data;
mod font;
mod key;
mod layout;
mod shape;
mod units;

pub use self::blend_mode::*;
pub use self::color::*;
pub use self::data::*;
pub use self::font::*;
pub use self::key::*;
pub use self::layout::*;
pub use self::shape::*;
pub use self::units::*;

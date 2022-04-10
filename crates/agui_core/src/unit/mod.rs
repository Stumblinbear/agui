pub(crate) const MARGIN_OF_ERROR: f32 = 0.01;

mod blend_mode;
mod color;
mod font;
mod key;
mod layout;
mod shape;
mod units;

pub use self::blend_mode::*;
pub use self::color::*;
pub use self::font::*;
pub use self::key::*;
pub use self::layout::*;
pub use self::shape::*;
pub use self::units::*;

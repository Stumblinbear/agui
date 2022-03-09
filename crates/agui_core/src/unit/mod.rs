pub(crate) const MARGIN_OF_ERROR: f32 = 0.01;

mod key;
mod layout;
mod render;
mod shape;
mod units;

pub use self::key::*;
pub use self::layout::*;
pub use self::render::*;
pub use self::shape::*;
pub use self::units::*;

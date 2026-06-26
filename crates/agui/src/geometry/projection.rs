use peniko::kurbo::Affine;

use crate::geometry::Offset;

/// A paint transform that stays a plain [`Offset`] until a non-translation transform promotes it to a full
/// [`Affine`]. An offset-only chain therefore never builds an `Affine`.
#[derive(Clone, Copy)]
pub(crate) enum Projection {
    Offset(Offset),
    Affine(Affine),
}

impl Default for Projection {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Projection {
    pub const IDENTITY: Self = Self::Offset(Offset::ZERO);

    /// This projection followed by a translation of `offset`.
    pub fn then_offset(self, offset: Offset) -> Self {
        match self {
            Self::Offset(accumulated) => Self::Offset(accumulated + offset),
            Self::Affine(affine) => Self::Affine(affine * Affine::translate(offset)),
        }
    }

    /// This projection followed by `transform`, promoting to a full affine.
    pub fn then_affine(self, transform: Affine) -> Self {
        let base = match self {
            Self::Offset(offset) => Affine::translate(offset),
            Self::Affine(affine) => affine,
        };

        Self::Affine(base * transform)
    }
}

impl From<Projection> for Affine {
    fn from(projection: Projection) -> Self {
        match projection {
            Projection::Offset(offset) => Affine::translate(offset),
            Projection::Affine(affine) => affine,
        }
    }
}

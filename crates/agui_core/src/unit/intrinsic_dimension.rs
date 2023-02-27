use super::Axis;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IntrinsicDimension {
    /// Calculate the minimum allowable width without failing to correctly paint
    /// its contents within itself without clipping.
    MinWidth,

    /// Calculate the smallest width beyond which increasing the width never decreases
    /// the preferred height. The preferred height is the value that would be the
    /// [`IntrinsicDimension::MinHeight`] value for that width.
    MaxWidth,

    /// Calculate the minimum allowable height without failing to correctly paint
    /// its contents within itself without clipping.
    MinHeight,

    /// Calculate the smallest height beyond which increasing the height never decreases
    /// the preferred width. The preferred width is the value that would be the
    /// [`IntrinsicDimension::MinWidth`] value for that height.
    MaxHeight,
}

impl IntrinsicDimension {
    pub fn axis(self) -> Axis {
        match self {
            IntrinsicDimension::MinWidth | IntrinsicDimension::MaxWidth => Axis::Horizontal,
            IntrinsicDimension::MinHeight | IntrinsicDimension::MaxHeight => Axis::Vertical,
        }
    }
}

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
    pub fn min_axis(axis: Axis) -> Self {
        match axis {
            Axis::Horizontal => IntrinsicDimension::MinWidth,
            Axis::Vertical => IntrinsicDimension::MinHeight,
        }
    }

    pub fn max_axis(axis: Axis) -> Self {
        match axis {
            Axis::Horizontal => IntrinsicDimension::MaxWidth,
            Axis::Vertical => IntrinsicDimension::MaxHeight,
        }
    }

    pub fn axis(self) -> Axis {
        Axis::from(self)
    }

    pub fn is_width(self) -> bool {
        match self {
            IntrinsicDimension::MinWidth | IntrinsicDimension::MaxWidth => true,
            IntrinsicDimension::MinHeight | IntrinsicDimension::MaxHeight => false,
        }
    }

    pub fn is_height(self) -> bool {
        match self {
            IntrinsicDimension::MinWidth | IntrinsicDimension::MaxWidth => false,
            IntrinsicDimension::MinHeight | IntrinsicDimension::MaxHeight => true,
        }
    }

    pub fn is_min(self) -> bool {
        match self {
            IntrinsicDimension::MinWidth | IntrinsicDimension::MinHeight => true,
            IntrinsicDimension::MaxWidth | IntrinsicDimension::MaxHeight => false,
        }
    }

    pub fn is_max(self) -> bool {
        match self {
            IntrinsicDimension::MinWidth | IntrinsicDimension::MinHeight => false,
            IntrinsicDimension::MaxWidth | IntrinsicDimension::MaxHeight => true,
        }
    }

    pub fn flip_axis(self) -> Self {
        match self {
            IntrinsicDimension::MinWidth => IntrinsicDimension::MinHeight,
            IntrinsicDimension::MaxWidth => IntrinsicDimension::MaxHeight,
            IntrinsicDimension::MinHeight => IntrinsicDimension::MinWidth,
            IntrinsicDimension::MaxHeight => IntrinsicDimension::MaxWidth,
        }
    }
}

impl From<IntrinsicDimension> for Axis {
    fn from(dimension: IntrinsicDimension) -> Self {
        match dimension {
            IntrinsicDimension::MinWidth | IntrinsicDimension::MaxWidth => Axis::Horizontal,
            IntrinsicDimension::MinHeight | IntrinsicDimension::MaxHeight => Axis::Vertical,
        }
    }
}

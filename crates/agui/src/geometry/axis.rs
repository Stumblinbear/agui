#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Axis {
    #[default]
    Horizontal,
    Vertical,
}

impl Axis {
    pub fn flip(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

/// A direction along the horizontal or vertical [`Axis`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxisDirection {
    /// Vertical, increasing upward.
    Up,
    /// Horizontal, increasing rightward.
    Right,
    /// Vertical, increasing downward.
    Down,
    /// Horizontal, increasing leftward.
    Left,
}

impl AxisDirection {
    /// The [`Axis`] this direction runs along.
    pub fn axis(self) -> Axis {
        match self {
            Self::Up | Self::Down => Axis::Vertical,
            Self::Left | Self::Right => Axis::Horizontal,
        }
    }

    /// The opposite direction along the same axis.
    pub fn reverse(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    /// Whether this direction points up or left, the reverse of the default down-and-right
    /// orientation.
    pub fn is_reversed(self) -> bool {
        matches!(self, Self::Up | Self::Left)
    }
}

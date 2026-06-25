use crate::text::TextBaseline;

mod column;
mod flexible;

pub use column::*;
pub use flexible::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MainAxisSize {
    /// Shrink to fit the children along the main axis, within the incoming constraints. A non-zero minimum
    /// extent can still leave free space.
    Min,

    /// Expand to fill the available main-axis space, within the incoming constraints.
    #[default]
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MainAxisAlignment {
    Start,
    #[default]
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CrossAxisAlignment {
    #[default]
    Start,

    End,

    Center,

    Stretch,

    Baseline(TextBaseline),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VerticalDirection {
    Up,

    #[default]
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlexFit {
    #[default]
    /// The child is forced to fill the available space.
    Tight,

    /// The child can be at most as large as the available space (but is allowed to be smaller).
    Loose,
}

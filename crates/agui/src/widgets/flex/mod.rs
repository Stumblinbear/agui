use std::cell::Cell;

use crate::{prelude::render_object::LayoutScope, text::TextBaseline};

mod column;
mod flexible;
mod render_flex;
mod row;

pub use column::*;
pub use flexible::*;
pub use render_flex::*;
pub use row::*;

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

/// How a flex child competes for main-axis space, read by [`RenderFlex`] to lay the child out.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FlexData {
    /// The child's share of the free main-axis space, relative to its flex siblings. Zero leaves the child
    /// inflexible, sized to its own content.
    pub flex: f32,

    /// Whether the child fills the main-axis space its flex factor wins, or may be smaller.
    pub fit: FlexFit,
}

/// The parent data a flex child reports up to its flex container: its [`FlexData`], plus a slot the container
/// writes its own relayout scope into during layout, which the child reads back to re-lay the container when
/// its flex changes.
pub(crate) struct FlexParentData {
    pub data: FlexData,
    pub container_scope: Cell<LayoutScope>,
}

impl FlexParentData {
    pub fn new(data: FlexData) -> Self {
        Self {
            data,
            container_scope: Cell::new(LayoutScope::detached()),
        }
    }
}

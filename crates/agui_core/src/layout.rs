use crate::unit::{Margin, Position, Sizing};

/// Holds layout parameters to dictate how the element should be displayed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layout {
    pub position: Position,
    pub min_sizing: Sizing,
    pub max_sizing: Sizing,
    pub sizing: Sizing,

    pub margin: Margin,
}

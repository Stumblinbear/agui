use crate::{
    unit::{Margin, Position, Sizing},
    Ref,
};

/// Holds layout parameters to dictate how the element should be displayed.
#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub position: Position,
    pub min_sizing: Sizing,
    pub max_sizing: Sizing,
    pub sizing: Sizing,

    pub margin: Margin,
}

impl From<Layout> for Ref<Layout> {
    fn from(layout: Layout) -> Self {
        Self::new(layout)
    }
}

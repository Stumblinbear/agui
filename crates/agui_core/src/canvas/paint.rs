use crate::unit::Color;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Paint {
    pub color: Color,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Brush(usize);

impl From<usize> for Brush {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

#[allow(clippy::from_over_into)]
impl Into<usize> for Brush {
    fn into(self) -> usize {
        self.0
    }
}

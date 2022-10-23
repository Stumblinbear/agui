use crate::unit::{BlendMode, Color};

#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct Paint {
    pub anti_alias: bool,
    pub color: Color,
    pub blend_mode: BlendMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Brush(usize);

impl Brush {
    pub fn idx(&self) -> usize {
        self.0
    }
}

impl From<usize> for Brush {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

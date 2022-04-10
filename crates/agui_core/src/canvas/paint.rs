use crate::unit::{BlendMode, Color};

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Paint {
    pub anti_alias: bool,
    pub blend_mode: BlendMode,
    pub color: Color,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
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

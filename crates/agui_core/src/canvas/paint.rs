use crate::unit::Color;

#[derive(Debug, Clone, PartialEq)]
pub struct Paint {
    pub color: Color,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Brush(pub(crate) usize);

use std::marker::PhantomData;

use crate::unit::{BlendMode, Color};

#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct Paint {
    pub anti_alias: bool,
    pub color: Color,
    pub blend_mode: BlendMode,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Brush<State> {
    pub(super) phantom: PhantomData<State>,

    pub(super) idx: usize,
}

impl<State> Brush<State> {
    pub fn idx(&self) -> usize {
        self.idx
    }
}

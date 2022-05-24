use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use crate::canvas::command::CanvasCommand;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RenderElementId(u64);

impl RenderElementId {
    pub fn from(element: &RenderElement) -> Self {
        let mut hasher = DefaultHasher::new();
        element.commands.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl std::fmt::Display for RenderElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl std::fmt::Debug for RenderElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

#[derive(Default)]
pub struct RenderElement {
    pub commands: Vec<CanvasCommand>,
}

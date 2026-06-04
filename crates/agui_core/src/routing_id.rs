use std::{borrow::Borrow, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoutingId(u16);

impl RoutingId {
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    /// Routing id for the child at `index`.
    ///
    /// # Panics
    ///
    /// Panics if `index` exceeds [`u16::MAX`].
    pub fn from_index(index: usize) -> Self {
        Self(u16::try_from(index).expect("a widget cannot address more than u16::MAX children"))
    }

    pub const fn next(&mut self) -> Self {
        let id = self.0;
        self.0 += 1;
        Self(id)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoutingPath(Arc<[RoutingId]>);

impl RoutingPath {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_descendant_of(&self, other: &Self) -> bool {
        self.0.starts_with(&other.0)
    }

    pub fn as_slice(&self) -> &[RoutingId] {
        &self.0
    }

    pub fn to_vec(&self) -> Vec<RoutingId> {
        self.0.to_vec()
    }
}

impl From<Vec<RoutingId>> for RoutingPath {
    fn from(path: Vec<RoutingId>) -> Self {
        Self(path.into())
    }
}

impl Borrow<[RoutingId]> for RoutingPath {
    fn borrow(&self) -> &[RoutingId] {
        &self.0
    }
}

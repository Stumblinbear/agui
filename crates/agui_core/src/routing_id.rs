use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoutingId(u16);

impl RoutingId {
    pub const fn new(id: u16) -> Self {
        Self(id)
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

impl From<Vec<RoutingId>> for RoutingPath {
    fn from(path: Vec<RoutingId>) -> Self {
        Self(path.into())
    }
}

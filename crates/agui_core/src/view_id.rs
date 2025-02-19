use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ViewId(u16);

impl ViewId {
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
pub struct ViewPath(Arc<[ViewId]>);

impl From<Vec<ViewId>> for ViewPath {
    fn from(path: Vec<ViewId>) -> Self {
        Self(path.into())
    }
}

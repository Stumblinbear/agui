#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextureId(Option<usize>);

impl TextureId {
    pub fn new(idx: usize) -> Self {
        Self(Some(idx))
    }

    pub fn idx(&self) -> Option<usize> {
        self.0
    }
}

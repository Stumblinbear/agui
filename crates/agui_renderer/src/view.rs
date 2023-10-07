#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RenderViewId(Option<usize>);

impl RenderViewId {
    pub(crate) fn new(id: usize) -> Self {
        Self(Some(id))
    }
}

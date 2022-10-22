use crate::unit::{Layout, LayoutType};

use super::WidgetRef;

#[derive(Default)]
pub struct LayoutResult {
    pub layout_type: LayoutType,
    pub layout: Layout,
}

#[derive(Default)]
pub struct BuildResult(Vec<WidgetRef>);

impl BuildResult {
    pub fn empty() -> Self {
        Self::default()
    }

    pub(crate) fn take(self) -> Vec<WidgetRef> {
        self.0
    }
}

impl From<WidgetRef> for BuildResult {
    fn from(widget: WidgetRef) -> Self {
        BuildResult::from([widget])
    }
}

impl From<&WidgetRef> for BuildResult {
    fn from(widget: &WidgetRef) -> Self {
        BuildResult::from([widget])
    }
}

impl<W, I> From<I> for BuildResult
where
    W: Into<WidgetRef>,
    I: IntoIterator<Item = W>,
{
    fn from(iter: I) -> Self {
        BuildResult(iter.into_iter().map(|w| w.into()).collect())
    }
}

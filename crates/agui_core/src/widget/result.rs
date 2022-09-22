use super::WidgetRef;

#[derive(Default)]
pub struct BuildResult {
    pub children: Vec<WidgetRef>,
}

impl BuildResult {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn with_children<W>(children: impl IntoIterator<Item = W>) -> Self
    where
        W: Into<WidgetRef>,
    {
        Self {
            children: children.into_iter().map(|w| w.into()).collect(),
        }
    }
}

impl From<WidgetRef> for BuildResult {
    fn from(widget: WidgetRef) -> Self {
        BuildResult::with_children([widget])
    }
}

impl From<&WidgetRef> for BuildResult {
    fn from(widget: &WidgetRef) -> Self {
        BuildResult::with_children([widget])
    }
}

impl<W, I> From<I> for BuildResult
where
    W: Into<WidgetRef>,
    I: IntoIterator<Item = W>,
{
    fn from(iter: I) -> Self {
        BuildResult::with_children(iter)
    }
}

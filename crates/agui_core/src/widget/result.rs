use super::Widget;

#[derive(Default)]
pub struct BuildResult {
    pub children: Vec<Widget>,
}

impl BuildResult {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn with_children<W>(children: impl IntoIterator<Item = W>) -> Self
    where
        W: Into<Widget>,
    {
        Self {
            children: children.into_iter().map(|w| w.into()).collect(),
        }
    }
}

impl From<Widget> for BuildResult {
    fn from(widget: Widget) -> Self {
        BuildResult::with_children([widget])
    }
}

impl From<&Widget> for BuildResult {
    fn from(widget: &Widget) -> Self {
        BuildResult::with_children([widget])
    }
}

impl<W, I> From<I> for BuildResult
where
    W: Into<Widget>,
    I: IntoIterator<Item = W>,
{
    fn from(iter: I) -> Self {
        BuildResult::with_children(iter)
    }
}

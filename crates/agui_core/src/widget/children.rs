use super::WidgetRef;

#[derive(Default)]
pub struct Children {
    children: Vec<WidgetRef>,
}

impl Children {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn new(widget: WidgetRef) -> Self {
        Self {
            children: vec![widget],
        }
    }

    pub(crate) fn take(self) -> Vec<WidgetRef> {
        self.children
    }
}

impl From<WidgetRef> for Children {
    fn from(widget: WidgetRef) -> Self {
        Children::from([widget])
    }
}

impl From<&WidgetRef> for Children {
    fn from(widget: &WidgetRef) -> Self {
        Children::from([widget])
    }
}

impl<W, I> From<I> for Children
where
    W: Into<WidgetRef>,
    I: IntoIterator<Item = W>,
{
    fn from(iter: I) -> Self {
        Children {
            children: iter.into_iter().map(|w| w.into()).collect(),
        }
    }
}

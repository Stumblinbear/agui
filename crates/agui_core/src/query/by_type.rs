use std::marker::PhantomData;

use crate::widget::{BoxedWidget, WidgetBuilder, WidgetElement};

#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Clone)]
pub struct QueryByType<I, W>
where
    W: WidgetBuilder,
{
    pub(crate) iter: I,
    phantom: PhantomData<W>,
}

impl<I, W> QueryByType<I, W>
where
    W: WidgetBuilder,
{
    pub(super) fn new(iter: I) -> Self {
        Self {
            iter,
            phantom: PhantomData,
        }
    }
}

impl<'query, I, W> Iterator for QueryByType<I, W>
where
    W: WidgetBuilder + 'query,
    I: Iterator<Item = &'query BoxedWidget>,
{
    type Item = &'query WidgetElement<W>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|widget| widget.downcast_ref())
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper) // can't know a lower bound
    }
}

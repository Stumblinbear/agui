use std::{cell::Ref, marker::PhantomData};

use crate::{
    manager::widget::{Widget, WidgetElement},
    widget::WidgetBuilder,
};

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
    pub(in crate::manager::query) fn new(iter: I) -> Self {
        Self {
            iter,
            phantom: PhantomData,
        }
    }
}

impl<'query, I, W> Iterator for QueryByType<I, W>
where
    W: WidgetBuilder,
    I: Iterator<Item = &'query Widget>,
{
    type Item = Ref<'query, WidgetElement<W>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|widget| widget.get_as::<W>())
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper) // can't know a lower bound
    }
}

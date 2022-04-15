use crate::{unit::Key, widget::Widget};

#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Clone)]
pub struct QueryByKey<I> {
    pub(crate) iter: I,
    key: Key,
}

impl<I> QueryByKey<I> {
    pub(in crate::engine::query) fn new(iter: I, key: Key) -> Self {
        Self { iter, key }
    }
}

impl<'query, I> Iterator for QueryByKey<I>
where
    I: Iterator<Item = &'query Widget>,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find(|widget| match widget {
            Widget::None => false,
            Widget::Some { key, .. } => key.filter(|key| key.get_key() == self.key).is_some(),
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper) // can't know a lower bound
    }
}

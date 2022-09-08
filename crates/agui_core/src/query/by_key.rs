use crate::{unit::Key, widget::BoxedWidget};

#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Clone)]
pub struct QueryByKey<I> {
    pub(crate) iter: I,
    key: Key,
}

impl<I> QueryByKey<I> {
    pub(in super) fn new(iter: I, key: Key) -> Self {
        Self { iter, key }
    }
}

impl<'query, I> Iterator for QueryByKey<I>
where
    I: Iterator<Item = &'query BoxedWidget>,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find(|widget| {
            widget
                .get_key()
                .filter(|key| key.get_key() == self.key)
                .is_some()
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper) // can't know a lower bound
    }
}

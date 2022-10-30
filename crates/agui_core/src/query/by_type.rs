use std::marker::PhantomData;

use crate::{element::Element, widget::Widget};

#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Clone)]
pub struct QueryByType<I, W>
where
    W: Widget,
{
    pub(crate) iter: I,
    phantom: PhantomData<W>,
}

impl<I, W> QueryByType<I, W>
where
    W: Widget,
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
    W: Widget + 'query,
    I: Iterator<Item = &'query Element>,
{
    type Item = &'query Element;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .find(|element| element.get_widget::<W>().is_some())
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper) // can't know a lower bound
    }
}

#[cfg(test)]
mod tests {
    use agui_macros::StatelessWidget;

    use crate::{
        manager::WidgetManager,
        query::WidgetQueryExt,
        widget::{BuildContext, BuildResult, WidgetRef, WidgetView},
    };

    #[derive(Default, StatelessWidget)]
    struct TestWidget1 {
        pub children: Vec<WidgetRef>,
    }

    impl PartialEq for TestWidget1 {
        fn eq(&self, _: &Self) -> bool {
            false
        }
    }

    impl WidgetView for TestWidget1 {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            (&self.children).into()
        }
    }

    #[derive(Default, StatelessWidget)]
    struct TestWidget2 {
        pub children: Vec<WidgetRef>,
    }

    impl PartialEq for TestWidget2 {
        fn eq(&self, _: &Self) -> bool {
            false
        }
    }

    impl WidgetView for TestWidget2 {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            (&self.children).into()
        }
    }

    #[test]
    pub fn finds_widget_by_type() {
        let mut manager = WidgetManager::with_root(TestWidget1 {
            children: [
                TestWidget2 {
                    ..Default::default()
                }
                .into(),
                TestWidget1 {
                    ..Default::default()
                }
                .into(),
            ]
            .into(),
        });

        manager.update();

        assert_eq!(
            manager.query().by_type::<TestWidget1>().count(),
            2,
            "should have found 2 widgets of type TestWidget1"
        );

        assert_eq!(
            manager.query().by_type::<TestWidget2>().count(),
            1,
            "should have found 1 widget of type TestWidget2"
        );
    }
}

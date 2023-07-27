use std::marker::PhantomData;

use crate::{element::Element, widget::AnyWidget};

#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Clone)]
pub struct QueryByType<I, W>
where
    W: AnyWidget,
{
    pub(crate) iter: I,
    phantom: PhantomData<W>,
}

impl<I, W> QueryByType<I, W>
where
    W: AnyWidget,
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
    W: AnyWidget + 'query,
    I: Iterator<Item = &'query Element>,
{
    type Item = &'query Element;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .find(|element| element.get_widget().downcast::<W>().is_some())
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
        widget::{BuildContext, Widget, WidgetBuild},
    };

    #[derive(Default, StatelessWidget)]
    struct TestWidget1 {
        pub child: Option<Widget>,
    }

    impl WidgetBuild for TestWidget1 {
        type Child = Option<Widget>;

        fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
            self.child.clone()
        }
    }

    #[derive(Default, StatelessWidget)]
    struct TestWidget2 {
        pub child: Option<Widget>,
    }

    impl WidgetBuild for TestWidget2 {
        type Child = Option<Widget>;

        fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
            self.child.clone()
        }
    }

    #[test]
    pub fn finds_widget_by_type() {
        let mut manager = WidgetManager::with_root(TestWidget1 {
            child: TestWidget2 {
                child: TestWidget1 {
                    ..Default::default()
                }
                .into(),
            }
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

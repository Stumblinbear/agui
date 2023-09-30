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
    use agui_macros::LayoutWidget;

    use crate::{
        engine::Engine,
        query::WidgetQueryExt,
        unit::Size,
        widget::{BuildContext, IntoWidget, Widget, WidgetLayout},
    };

    #[derive(Default, LayoutWidget)]
    struct TestWidget1 {
        pub child: Option<Widget>,
    }

    impl WidgetLayout for TestWidget1 {
        fn build(&self, _: &mut BuildContext<Self>) -> Vec<Widget> {
            self.child.clone().into_iter().collect()
        }

        fn layout(
            &self,
            _: &mut crate::widget::LayoutContext,
            _: crate::unit::Constraints,
        ) -> Size {
            Size::ZERO
        }
    }

    #[derive(Default, LayoutWidget)]
    struct TestWidget2 {
        pub child: Option<Widget>,
    }

    impl WidgetLayout for TestWidget2 {
        fn build(&self, _: &mut BuildContext<Self>) -> Vec<Widget> {
            self.child.clone().into_iter().collect()
        }

        fn layout(
            &self,
            _: &mut crate::widget::LayoutContext,
            _: crate::unit::Constraints,
        ) -> Size {
            Size::ZERO
        }
    }

    #[test]
    pub fn finds_widget_by_type() {
        let mut engine = Engine::builder()
            .with_root(TestWidget1 {
                child: Some(
                    TestWidget2 {
                        child: Some(
                            TestWidget1 {
                                ..Default::default()
                            }
                            .into_widget(),
                        ),
                    }
                    .into_widget(),
                ),
            })
            .build();

        engine.update();

        assert_eq!(
            engine.query().by_type::<TestWidget1>().count(),
            2,
            "should have found 2 widgets of type TestWidget1"
        );

        assert_eq!(
            engine.query().by_type::<TestWidget2>().count(),
            1,
            "should have found 1 widget of type TestWidget2"
        );
    }
}

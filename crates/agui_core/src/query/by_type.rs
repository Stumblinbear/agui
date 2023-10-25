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
            .find(|element| element.widget().downcast::<W>().is_some())
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper) // can't know a lower bound
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        element::mock::{build::MockBuildWidget, proxy::MockProxyWidget, DummyWidget},
        engine::Engine,
        query::WidgetQueryExt,
        widget::IntoWidget,
    };

    #[test]
    pub fn finds_widget_by_type() {
        let proxy_widget = MockProxyWidget::default();
        {
            proxy_widget.mock.borrow_mut().expect_child().returning(|| {
                let build_widget = MockBuildWidget::default();
                {
                    build_widget
                        .mock
                        .borrow_mut()
                        .expect_build()
                        .returning(|_| {
                            let build_widget = MockBuildWidget::default();
                            {
                                build_widget
                                    .mock
                                    .borrow_mut()
                                    .expect_build()
                                    .returning(|_| DummyWidget.into_widget());
                            }
                            build_widget.into_widget()
                        });
                }
                build_widget.into_widget()
            });
        }

        let mut engine = Engine::builder().with_root(proxy_widget).build();

        engine.update();

        assert_eq!(
            engine.query().by_type::<MockProxyWidget>().count(),
            1,
            "should have found 1 widget of type MockProxyWidget"
        );

        assert_eq!(
            engine.query().by_type::<MockBuildWidget>().count(),
            2,
            "should have found 2 widgets of type MockBuildWidget"
        );

        assert_eq!(
            engine.query().by_type::<DummyWidget>().count(),
            1,
            "should have found 1 widget of type DummyWidget"
        );
    }
}

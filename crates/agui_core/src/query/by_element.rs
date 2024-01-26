use std::marker::PhantomData;

use crate::element::{lifecycle::ElementLifecycle, Element, ElementId};

#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Clone)]
pub struct QueryByElement<I, E> {
    phantom: PhantomData<E>,

    pub(crate) iter: I,
}

impl<I, E> QueryByElement<I, E> {
    pub(super) fn new(iter: I) -> Self {
        Self {
            phantom: PhantomData,

            iter,
        }
    }
}

impl<'query, I, E> Iterator for QueryByElement<I, E>
where
    E: ElementLifecycle + 'query,
    I: Iterator<Item = (ElementId, &'query Element)>,
{
    type Item = (ElementId, &'query Element);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .find(|(_, element)| element.downcast::<E>().is_some())
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
        element::mock::{
            build::{MockBuildWidget, MockedElementBuild},
            render::{MockRenderWidget, MockedElementRender},
            DummyWidget,
        },
        engine::widgets::WidgetManager,
        query::WidgetQueryExt,
        widget::IntoWidget,
    };

    #[test]
    pub fn finds_widget_by_type() {
        let root_widget = MockRenderWidget::default();
        {
            root_widget
                .mock
                .borrow_mut()
                .expect_children()
                .returning(|| {
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
                    vec![build_widget.into_widget()]
                });
        }

        let mut manager = WidgetManager::with_root(root_widget);

        manager.update();

        assert_eq!(
            manager.query().by_element::<MockedElementRender>().count(),
            1,
            "should have found 1 element of type MockedElementRender"
        );

        assert_eq!(
            manager.query().by_element::<MockedElementBuild>().count(),
            2,
            "should have found 2 elements of type MockedElementBuild"
        );

        assert_eq!(
            manager.query().by_element::<MockedElementRender>().count(),
            1,
            "should have found 1 element of type MockedElementRender"
        );
    }
}

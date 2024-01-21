use std::marker::PhantomData;

use crate::{
    element::{Element, ElementBuilder},
    widget::AnyWidget,
};

#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Clone)]
pub struct QueryByWidget<I, W> {
    pub(crate) iter: I,
    phantom: PhantomData<W>,
}

impl<I, W> QueryByWidget<I, W> {
    pub(super) fn new(iter: I) -> Self {
        Self {
            iter,
            phantom: PhantomData,
        }
    }
}

impl<'query, I, W> Iterator for QueryByWidget<I, W>
where
    W: AnyWidget + ElementBuilder + 'query,
    I: Iterator<Item = &'query Element>,
{
    type Item = &'query Element;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find(|element| {
            element
                .downcast::<<W as ElementBuilder>::Element>()
                .is_some()
        })
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
        element::mock::{build::MockBuildWidget, render::MockRenderWidget, DummyWidget},
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
            manager.query().by_widget::<MockRenderWidget>().count(),
            1,
            "should have found 1 widget of type MockRenderWidget"
        );

        assert_eq!(
            manager.query().by_widget::<MockBuildWidget>().count(),
            2,
            "should have found 2 widgets of type MockBuildWidget"
        );

        assert_eq!(
            manager.query().by_widget::<DummyWidget>().count(),
            1,
            "should have found 1 widget of type DummyWidget"
        );
    }
}
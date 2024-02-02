use std::{marker::PhantomData, rc::Rc};

use crate::{
    element::{widget::ElementWidget, ElementBuilder},
    engine::elements::{
        iter::{ElementEntry, ElementTreeIterator},
        ElementTree,
    },
    widget::AnyWidget,
};

pub trait FilterByWidgetExt {
    fn filter_widget<W>(self) -> FilterWidget<W, Self>
    where
        Self: Sized,
    {
        FilterWidget::new(self)
    }
}

impl<I> FilterByWidgetExt for I where I: ElementTreeIterator {}

pub trait ExactWidgetIterator {
    type Widget: AnyWidget + ElementBuilder;

    fn and_downcast(self) -> DowncastWidget<Self::Widget, Self>
    where
        Self: Sized,
    {
        DowncastWidget::new(self)
    }
}

#[derive(Clone)]
pub struct FilterWidget<W, I> {
    phantom: PhantomData<W>,

    inner: I,
}

impl<W, I> FilterWidget<W, I> {
    pub fn new(inner: I) -> Self {
        Self {
            phantom: PhantomData,

            inner,
        }
    }
}

impl<'query, W, I> Iterator for FilterWidget<W, I>
where
    W: AnyWidget + ElementBuilder,
    <W as ElementBuilder>::Element: ElementWidget,
    I: ElementTreeIterator<Item = ElementEntry<'query>>,
{
    type Item = ElementEntry<'query>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.find(|node| {
            node.element()
                .downcast::<<W as ElementBuilder>::Element>()
                .and_then(|element| Rc::clone(element.widget()).as_any().downcast::<W>().ok())
                .is_some()
        })
    }
}

impl<'query, W, I> ElementTreeIterator for FilterWidget<W, I>
where
    W: AnyWidget + ElementBuilder,
    <W as ElementBuilder>::Element: ElementWidget,
    I: ElementTreeIterator<Item = ElementEntry<'query>>,
{
    fn tree(&self) -> &ElementTree {
        self.inner.tree()
    }
}

impl<W, I> ExactWidgetIterator for FilterWidget<W, I>
where
    W: AnyWidget + ElementBuilder,
    <W as ElementBuilder>::Element: ElementWidget,
{
    type Widget = W;
}

#[derive(Clone)]
pub struct DowncastWidget<W, I> {
    phantom: PhantomData<W>,

    inner: I,
}

impl<W, I> DowncastWidget<W, I> {
    pub fn new(inner: I) -> Self {
        Self {
            phantom: PhantomData,

            inner,
        }
    }
}

impl<'query, W, I> Iterator for DowncastWidget<W, I>
where
    W: AnyWidget + ElementBuilder,
    <W as ElementBuilder>::Element: ElementWidget,
    I: ElementTreeIterator<Item = ElementEntry<'query>>,
{
    type Item = Rc<W>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|node| {
            Rc::clone(
                node.element()
                    .downcast::<<W as ElementBuilder>::Element>()
                    .expect("element downcast failed")
                    .widget(),
            )
            .as_any()
            .downcast::<W>()
            .expect("widget downcast failed")
        })
    }
}

impl<'query, W, I> ElementTreeIterator for DowncastWidget<W, I>
where
    W: AnyWidget + ElementBuilder,
    <W as ElementBuilder>::Element: ElementWidget,
    I: ElementTreeIterator<Item = ElementEntry<'query>>,
{
    fn tree(&self) -> &ElementTree {
        self.inner.tree()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        element::mock::{build::MockBuildWidget, render::MockRenderWidget},
        engine::elements::{strategies::mocks::MockInflateElementStrategy, ElementTree},
        query::by_widget::{ExactWidgetIterator, FilterByWidgetExt},
        widget::IntoWidget,
    };

    #[test]
    pub fn finds_widget_by_type() {
        let root_widget = MockRenderWidget::default();
        {
            root_widget.mock().expect_children().returning(|| {
                let build_widget = MockBuildWidget::default();
                {
                    build_widget
                        .mock
                        .borrow_mut()
                        .expect_build()
                        .returning(|_| MockRenderWidget::dummy());
                }
                vec![build_widget.into_widget()]
            });
        }

        let mut tree = ElementTree::default();

        tree.inflate(
            &mut MockInflateElementStrategy::default(),
            None,
            root_widget.into_widget(),
        )
        .expect("failed to spawn and inflate");

        assert_eq!(
            tree.iter().filter_widget::<MockRenderWidget>().count(),
            2,
            "should have found 2 widgets of type MockRenderWidget"
        );

        assert_eq!(
            tree.iter().filter_widget::<MockBuildWidget>().count(),
            1,
            "should have found 1 widget of type MockBuildWidget"
        );
    }

    #[test]
    pub fn downcasts_widgets() {
        let root_widget = MockRenderWidget::default();
        {
            root_widget.mock().expect_children().returning(|| {
                let build_widget = MockBuildWidget::default();
                {
                    build_widget
                        .mock
                        .borrow_mut()
                        .expect_build()
                        .returning(|_| MockRenderWidget::dummy());
                }
                vec![build_widget.into_widget()]
            });
        }

        let mut tree = ElementTree::default();

        tree.inflate(
            &mut MockInflateElementStrategy::default(),
            None,
            root_widget.into_widget(),
        )
        .expect("failed to spawn and inflate");

        assert_eq!(
            tree.iter()
                .filter_widget::<MockRenderWidget>()
                .and_downcast()
                .count(),
            2,
            "should have found and downcasted 2 widgets of type MockRenderWidget"
        );

        assert_eq!(
            tree.iter()
                .filter_widget::<MockBuildWidget>()
                .and_downcast()
                .count(),
            1,
            "should have found and downcasted 1 widget of type MockBuildWidget"
        );
    }
}

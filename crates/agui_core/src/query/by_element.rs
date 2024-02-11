use std::marker::PhantomData;

use crate::{
    element::lifecycle::ElementLifecycle,
    engine::elements::{
        iter::{ElementEntry, ElementTreeIterator},
        ElementTree,
    },
};

pub trait FilterByElementExt {
    fn filter_element<E>(self) -> FilterElement<E, Self>
    where
        Self: ElementTreeIterator + Sized,
    {
        FilterElement::new(self)
    }
}

impl<I> FilterByElementExt for I where I: ElementTreeIterator {}

pub trait ExactElementIterator {
    type Element: ElementLifecycle;

    fn and_downcast(self) -> DowncastElement<Self::Element, Self>
    where
        Self: Sized,
    {
        DowncastElement::new(self)
    }
}

#[derive(Clone)]
pub struct FilterElement<E, I> {
    phantom: PhantomData<E>,

    inner: I,
}

impl<E, I> FilterElement<E, I> {
    pub fn new(inner: I) -> Self {
        Self {
            phantom: PhantomData,

            inner,
        }
    }
}

impl<'query, E, I> Iterator for FilterElement<E, I>
where
    E: ElementLifecycle,
    I: ElementTreeIterator<Item = ElementEntry<'query>>,
{
    type Item = ElementEntry<'query>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .find(|node| node.element().downcast::<E>().is_some())
    }
}

impl<'query, E, I> ElementTreeIterator for FilterElement<E, I>
where
    E: ElementLifecycle,
    I: ElementTreeIterator<Item = ElementEntry<'query>>,
{
    fn tree(&self) -> &ElementTree {
        self.inner.tree()
    }
}

impl<E, I> ExactElementIterator for FilterElement<E, I>
where
    E: ElementLifecycle,
{
    type Element = E;
}

#[derive(Clone)]
pub struct DowncastElement<E, I> {
    phantom: PhantomData<E>,

    inner: I,
}

impl<E, I> DowncastElement<E, I> {
    pub fn new(inner: I) -> Self {
        Self {
            phantom: PhantomData,

            inner,
        }
    }
}

impl<'query, E, I> Iterator for DowncastElement<E, I>
where
    E: ElementLifecycle,
    I: ElementTreeIterator<Item = ElementEntry<'query>>,
{
    type Item = &'query E;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.inner.next()?;

        Some(
            node.element()
                .downcast::<E>()
                .expect("element downcast failed"),
        )
    }
}

impl<'query, E, I> ElementTreeIterator for DowncastElement<E, I>
where
    E: ElementLifecycle,
    I: ElementTreeIterator<Item = ElementEntry<'query>>,
{
    fn tree(&self) -> &ElementTree {
        self.inner.tree()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        element::mock::{
            build::{MockBuildWidget, MockedElementBuild},
            render::{MockRenderWidget, MockedElementRender},
        },
        engine::elements::{strategies::mocks::MockInflateElements, ElementTree},
        query::by_element::{ExactElementIterator, FilterByElementExt},
        widget::IntoWidget,
    };

    #[test]
    pub fn finds_element_by_type() {
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
                vec![build_widget.into_widget(), MockRenderWidget::dummy()]
            });
        }

        let mut tree = ElementTree::default();

        tree.inflate(
            &mut MockInflateElements::default(),
            root_widget.into_widget(),
        )
        .expect("failed to spawn and inflate");

        assert_eq!(
            tree.iter().filter_element::<MockedElementRender>().count(),
            3,
            "should have found 1 element of type MockedElementRender"
        );

        assert_eq!(
            tree.iter().filter_element::<MockedElementBuild>().count(),
            1,
            "should have found 2 elements of type MockedElementBuild"
        );
    }

    #[test]
    pub fn downcasts_elements() {
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
                vec![build_widget.into_widget(), MockRenderWidget::dummy()]
            });
        }

        let mut tree = ElementTree::default();

        tree.inflate(
            &mut MockInflateElements::default(),
            root_widget.into_widget(),
        )
        .expect("failed to spawn and inflate");

        assert_eq!(
            tree.iter()
                .filter_element::<MockedElementRender>()
                .and_downcast()
                .count(),
            3,
            "should have found and downcasted 1 element of type MockedElementRender"
        );

        assert_eq!(
            tree.iter()
                .filter_element::<MockedElementBuild>()
                .and_downcast()
                .count(),
            1,
            "should have found and downcasted 2 elements of type MockedElementBuild"
        );
    }
}

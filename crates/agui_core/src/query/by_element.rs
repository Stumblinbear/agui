use std::marker::PhantomData;

use crate::{
    element::{lifecycle::ElementLifecycle, Element, ElementId},
    engine::elements::tree::ElementTree,
    util::tree::TreeNode,
};

#[derive(Clone)]
pub struct QueryByElement<'query, E> {
    tree: &'query ElementTree,
    phantom: PhantomData<E>,
}

impl<'query, E> QueryByElement<'query, E> {
    pub fn new(tree: &'query ElementTree) -> Self {
        Self {
            tree,
            phantom: PhantomData,
        }
    }
}

impl<'query, E> QueryByElement<'query, E>
where
    E: ElementLifecycle,
{
    pub fn iter_nodes(&self) -> impl Iterator<Item = (ElementId, &TreeNode<ElementId, Element>)> {
        self.tree.iter_nodes().filter(|(_, element)| {
            element
                .value()
                .expect("cannot query an element's type while it is in use")
                .downcast::<E>()
                .is_some()
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (ElementId, &Element)> {
        self.tree
            .iter()
            .filter(|(_, element)| element.downcast::<E>().is_some())
    }

    pub fn iter_downcast(&self) -> impl Iterator<Item = (ElementId, &E)> {
        self.tree.iter().filter_map(|(element_id, element)| {
            element.downcast::<E>().map(|element| (element_id, element))
        })
    }

    pub fn count(&self) -> usize {
        self.iter().count()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        element::mock::{
            build::{MockBuildWidget, MockedElementBuild},
            render::{MockRenderWidget, MockedElementRender},
        },
        engine::elements::{strategies::tests::MockInflateStrategy, tree::ElementTree},
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
                vec![build_widget.into_widget(), MockRenderWidget::dummy()]
            });
        }

        let mut element_tree = ElementTree::default();

        element_tree
            .spawn_and_inflate(
                &mut MockInflateStrategy::default(),
                None,
                root_widget.into_widget(),
            )
            .expect("failed to spawn and inflate");

        assert_eq!(
            element_tree
                .query()
                .by_element::<MockedElementRender>()
                .count(),
            3,
            "should have found 1 element of type MockedElementRender"
        );

        assert_eq!(
            element_tree
                .query()
                .by_element::<MockedElementBuild>()
                .count(),
            1,
            "should have found 2 elements of type MockedElementBuild"
        );
    }
}

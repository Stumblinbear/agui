use std::{marker::PhantomData, rc::Rc};

use crate::{
    element::{widget::ElementWidget, Element, ElementBuilder, ElementId},
    engine::elements::ElementTree,
    util::tree::TreeNode,
    widget::AnyWidget,
};

#[derive(Clone)]
pub struct QueryByWidget<'query, W> {
    tree: &'query ElementTree,
    phantom: PhantomData<W>,
}

impl<'query, W> QueryByWidget<'query, W> {
    pub fn new(tree: &'query ElementTree) -> Self {
        Self {
            tree,
            phantom: PhantomData,
        }
    }
}

impl<'query, W> QueryByWidget<'query, W>
where
    W: AnyWidget + ElementBuilder,
    <W as ElementBuilder>::Element: ElementWidget,
{
    pub fn iter_nodes(&self) -> impl Iterator<Item = (ElementId, &TreeNode<ElementId, Element>)> {
        self.tree.iter_nodes().filter(|(_, element)| {
            element
                .value()
                .expect("cannot query an element's widget type while it is in use")
                .downcast::<<W as ElementBuilder>::Element>()
                .and_then(|element| Rc::clone(element.widget()).as_any().downcast::<W>().ok())
                .is_some()
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (ElementId, &Element)> {
        self.tree.iter().filter(|(_, element)| {
            element
                .downcast::<<W as ElementBuilder>::Element>()
                .and_then(|element| Rc::clone(element.widget()).as_any().downcast::<W>().ok())
                .is_some()
        })
    }

    pub fn iter_downcast(&self) -> impl Iterator<Item = (ElementId, Rc<W>)> + '_ {
        self.tree.iter().filter_map(|(element_id, element)| {
            element
                .downcast::<<W as ElementBuilder>::Element>()
                .and_then(|element| Rc::clone(element.widget()).as_any().downcast::<W>().ok())
                .map(|widget| (element_id, widget))
        })
    }

    pub fn count(&self) -> usize {
        self.iter().count()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        element::mock::{build::MockBuildWidget, render::MockRenderWidget},
        engine::elements::{strategies::mocks::MockInflateElementStrategy, ElementTree},
        query::ElementQuery,
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

        let mut element_tree = ElementTree::default();

        element_tree
            .inflate(
                &mut MockInflateElementStrategy::default(),
                None,
                root_widget.into_widget(),
            )
            .expect("failed to spawn and inflate");

        assert_eq!(
            ElementQuery::new(&element_tree)
                .by_widget::<MockRenderWidget>()
                .count(),
            2,
            "should have found 2 widgets of type MockRenderWidget"
        );

        assert_eq!(
            ElementQuery::new(&element_tree)
                .by_widget::<MockBuildWidget>()
                .count(),
            1,
            "should have found 1 widget of type MockBuildWidget"
        );
    }
}

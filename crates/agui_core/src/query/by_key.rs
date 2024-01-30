use crate::{
    element::{Element, ElementId},
    engine::elements::tree::ElementTree,
    unit::Key,
    util::tree::TreeNode,
};

#[derive(Clone)]
pub struct QueryByKey<'query> {
    tree: &'query ElementTree,
    key: Key,
}

impl<'query> QueryByKey<'query> {
    pub(super) fn new(tree: &'query ElementTree, key: Key) -> Self {
        Self { tree, key }
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = (ElementId, &TreeNode<ElementId, Element>)> {
        // TODO: optimize for global keys
        self.tree
            .iter_nodes()
            .filter(|(element_id, _)| self.tree.keyed().get_key(*element_id) == Some(self.key))
    }

    pub fn iter(&self) -> impl Iterator<Item = (ElementId, &Element)> {
        // TODO: optimize for global keys
        self.tree
            .iter()
            .filter(|(element_id, _)| self.tree.keyed().get_key(*element_id) == Some(self.key))
    }

    pub fn count(&self) -> usize {
        self.iter().count()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        element::mock::{build::MockBuildWidget, render::MockRenderWidget},
        engine::elements::{strategies::tests::MockInflateStrategy, tree::ElementTree},
        unit::Key,
        widget::{IntoWidget, Widget},
    };

    #[test]
    pub fn finds_widget_by_key() {
        let root_widget = MockRenderWidget::default();
        {
            root_widget.mock().expect_children().returning(|| {
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
                                    .returning(|_| MockRenderWidget::dummy());
                            }
                            build_widget.into_widget()
                        });
                }
                vec![
                    Widget::new_with_key(Key::local(0), build_widget.clone()),
                    Widget::new_with_key(Key::local(1), build_widget.clone()),
                    Widget::new_with_key(Key::local(2), build_widget),
                    // Widget::new_with_key(Key::global(3), build_widget),
                ]
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
            element_tree.query().by_key(Key::local(0)).count(),
            1,
            "should have found 1 widget with local key 0"
        );

        assert_eq!(
            element_tree.query().by_key(Key::local(1)).count(),
            1,
            "should have found 1 widget with local key 1"
        );

        assert_eq!(
            element_tree.query().by_key(Key::local(3)).count(),
            0,
            "should have found 0 widgets with local key 3"
        );

        // assert_eq!(
        //     element_tree.query().by_key(Key::global(3)).count(),
        //     1,
        //     "should have found 1 widget with global key 3"
        // );
    }
}

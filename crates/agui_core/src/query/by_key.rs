use crate::{
    engine::elements::{
        iter::{ElementEntry, ElementTreeIterator},
        ElementTree,
    },
    unit::Key,
};

pub trait FilterKeyExt {
    fn filter_key(self, key: Key) -> FilterKey<Self>
    where
        Self: ElementTreeIterator + Sized,
    {
        FilterKey::new(key, self)
    }
}

impl<I> FilterKeyExt for I where I: ElementTreeIterator {}

#[derive(Clone)]
pub struct FilterKey<I> {
    key: Key,

    inner: I,
}

impl<I> FilterKey<I> {
    pub(super) fn new(key: Key, inner: I) -> Self {
        Self { key, inner }
    }
}

impl<'query, I> Iterator for FilterKey<I>
where
    I: ElementTreeIterator<Item = ElementEntry<'query>>,
{
    type Item = ElementEntry<'query>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.inner.next() {
            if self.inner.tree().keyed().get_key(node.id()) == Some(self.key) {
                return Some(node);
            }
        }

        None
    }
}

impl<'query, I> ElementTreeIterator for FilterKey<I>
where
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
        query::by_key::FilterKeyExt,
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

        let mut tree = ElementTree::default();

        tree.inflate(
            &mut MockInflateElementStrategy::default(),
            None,
            root_widget.into_widget(),
        )
        .expect("failed to spawn and inflate");

        assert_eq!(
            tree.iter().filter_key(Key::local(0)).count(),
            1,
            "should have found 1 widget with local key 0"
        );

        assert_eq!(
            tree.iter().filter_key(Key::local(1)).count(),
            1,
            "should have found 1 widget with local key 1"
        );

        assert_eq!(
            tree.iter().filter_key(Key::local(3)).count(),
            0,
            "should have found 0 widgets with local key 3"
        );

        // assert_eq!(
        //     tree.iter().by_key(Key::global(3)).count(),
        //     1,
        //     "should have found 1 widget with global key 3"
        // );
    }
}

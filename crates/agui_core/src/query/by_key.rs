use crate::{
    element::{Element, ElementId},
    query::WithWidgetKeyStorage,
    unit::Key,
};

#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Clone)]
pub struct QueryByKey<I> {
    pub(crate) iter: I,
    key: Key,
}

impl<I> QueryByKey<I> {
    pub(super) fn new(iter: I, key: Key) -> Self {
        Self { iter, key }
    }
}

impl<'query, I> Iterator for QueryByKey<I>
where
    I: Iterator<Item = (ElementId, &'query Element)> + WithWidgetKeyStorage,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.key {
            global_key @ Key::Global(_) => {
                let element_id = self.iter.get_element_key(global_key)?;

                self.iter.find(|(id, _)| *id == element_id)
            }

            local_key @ Key::Local(_) => {
                while let Some((element_id, element)) = self.iter.next() {
                    let Some(key) = self.iter.get_key(element_id) else {
                        continue;
                    };

                    if key == local_key {
                        return Some((element_id, element));
                    } else {
                        continue;
                    }
                }

                None
            }
        }
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
        unit::Key,
        widget::{IntoWidget, Widget},
    };

    #[test]
    pub fn finds_widget_by_key() {
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
                    vec![
                        Widget::new_with_key(Key::local(0), build_widget.clone()),
                        Widget::new_with_key(Key::local(1), build_widget.clone()),
                        Widget::new_with_key(Key::local(2), build_widget.clone()),
                        Widget::new_with_key(Key::global(3), build_widget),
                    ]
                });
        }

        let mut manager = WidgetManager::with_root(root_widget);

        manager.update();

        assert_eq!(
            manager.query().by_key(Key::local(0)).count(),
            1,
            "should have found 1 widget with local key 0"
        );

        assert_eq!(
            manager.query().by_key(Key::local(1)).count(),
            1,
            "should have found 1 widget with local key 1"
        );

        assert_eq!(
            manager.query().by_key(Key::local(3)).count(),
            0,
            "should have found 0 widgets with local key 3"
        );

        assert_eq!(
            manager.query().by_key(Key::global(3)).count(),
            1,
            "should have found 1 widget with global key 3"
        );
    }
}

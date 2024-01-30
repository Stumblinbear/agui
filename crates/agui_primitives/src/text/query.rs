use std::rc::Rc;

use agui_core::element::{widget::ElementWidget, Element, ElementBuilder, ElementId};

use crate::text::Text;

pub trait TextQueryExt<'query> {
    fn with_text(self, text: &str) -> QueryWithText<Self>
    where
        Self: Sized;
}

impl<'query, I> TextQueryExt<'query> for I
where
    I: Iterator<Item = (ElementId, &'query Element)>,
{
    fn with_text(self, text: &str) -> QueryWithText<Self>
    where
        Self: Sized,
    {
        QueryWithText::new(self, text)
    }
}

#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Clone)]
pub struct QueryWithText<'t, I> {
    pub(crate) iter: I,
    text: &'t str,
}

impl<'t, I> QueryWithText<'t, I> {
    fn new(iter: I, text: &'t str) -> Self {
        Self { iter, text }
    }
}

impl<'query, 't, I> Iterator for QueryWithText<'t, I>
where
    I: Iterator<Item = (ElementId, &'query Element)>,
{
    type Item = Rc<Text>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|(_, element)| {
            element
                .downcast::<<Text as ElementBuilder>::Element>()
                .map(|element| element.widget())
                .filter(|widget| widget.text == self.text)
                .map(Rc::clone)
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
    use agui_core::engine::widgets::WidgetManager;
    use agui_macros::build;

    use crate::{flex::Column, text::query::TextQueryExt, text::Text};

    #[test]
    pub fn finds_widget_with_text() {
        let mut manager = WidgetManager::default_with_root(build! {
            <Column> {
                children: [
                    <Text> {
                        text: "foo".into(),
                    }.into(),
                    <Text> {
                        text: "bar".into(),
                    }.into(),
                ],
            }
        });

        manager.update();

        assert_eq!(
            manager
                .query()
                .with_text("foo")
                .next()
                .expect("should have found a widget")
                .text,
            "foo",
            "should have found the \"foo\" text widget"
        );

        assert_eq!(
            manager
                .query()
                .with_text("bar")
                .next()
                .expect("should have found a widget")
                .text,
            "bar",
            "should have found the \"bar\" text widget"
        );
    }
}

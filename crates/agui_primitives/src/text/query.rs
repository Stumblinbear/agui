use std::cell::Ref;

use agui_core::{engine::widget::WidgetElement, widget::Widget};

use crate::Text;

pub trait TextQueryExt<'query> {
    fn with_text(self, text: &str) -> QueryWithText<Self>
    where
        Self: Sized;
}

impl<'query, I> TextQueryExt<'query> for I
where
    I: Iterator<Item = &'query Widget>,
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
    pub(in crate::text) fn new(iter: I, text: &'t str) -> Self {
        Self { iter, text }
    }
}

impl<'query, 't, I> Iterator for QueryWithText<'t, I>
where
    I: Iterator<Item = &'query Widget>,
{
    type Item = Ref<'query, WidgetElement<Text>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|widget| {
            widget
                .get_as::<Text>()
                .filter(|widget| widget.get_widget().text == self.text)
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
    use agui_core::engine::Engine;

    use crate::{query::TextQueryExt, Column, Text};

    #[test]
    pub fn finds_widget_with_text() {
        let mut engine = Engine::with_root(Column {
            children: vec![
                Text {
                    text: "foo".into(),
                    ..Default::default()
                }
                .into(),
                Text {
                    text: "bar".into(),
                    ..Default::default()
                }
                .into(),
            ],
            ..Default::default()
        });

        engine.update();

        assert_eq!(
            engine
                .query()
                .with_text("foo")
                .next()
                .expect("should have found a widget")
                .get_widget()
                .text,
            "foo",
            "should have found the \"foo\" text widget"
        );

        assert_eq!(
            engine
                .query()
                .with_text("bar")
                .next()
                .expect("should have found a widget")
                .get_widget()
                .text,
            "bar",
            "should have found the \"bar\" text widget"
        );
    }
}

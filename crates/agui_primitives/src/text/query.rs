use agui_core::{
    element::{widget::ElementWidget, ElementBuilder},
    engine::elements::{
        iter::{ElementEntry, ElementTreeIterator},
        ElementTree,
    },
    query::by_widget::ExactWidgetIterator,
};

use crate::text::Text;

pub trait FilterTextExt {
    fn with_text(self, text: &str) -> FilterText<Self>
    where
        Self: Sized,
    {
        FilterText::new(text, self)
    }
}

impl<I> FilterTextExt for I where I: ElementTreeIterator {}

#[derive(Clone)]
pub struct FilterText<'query, I> {
    text: &'query str,

    inner: I,
}

impl<'query, I> FilterText<'query, I> {
    pub fn new(text: &'query str, inner: I) -> Self {
        Self { text, inner }
    }
}

impl<'query, I> Iterator for FilterText<'query, I>
where
    I: ElementTreeIterator<Item = ElementEntry<'query>>,
{
    type Item = ElementEntry<'query>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.find(|node| {
            node.element()
                .downcast::<<Text as ElementBuilder>::Element>()
                .filter(|element| element.widget().text == self.text)
                .is_some()
        })
    }
}

impl<'query, I> ElementTreeIterator for FilterText<'query, I>
where
    I: ElementTreeIterator<Item = ElementEntry<'query>>,
{
    fn tree(&self) -> &ElementTree {
        self.inner.tree()
    }
}

impl<I> ExactWidgetIterator for FilterText<'_, I> {
    type Widget = Text;
}

#[cfg(test)]
mod tests {
    use agui_core::{
        engine::elements::{strategies::mocks::MockInflateElements, ElementTree},
        query::by_widget::ExactWidgetIterator,
        unit::{Color, Font, HorizontalAlign, TextStyle, VerticalAlign},
    };
    use agui_macros::build;

    use crate::{
        flex::Column,
        text::{query::FilterTextExt, Text},
    };

    #[test]
    pub fn finds_widget_with_text() {
        let mut tree = ElementTree::new();

        tree.inflate(
            &mut MockInflateElements::default(),
            build! {
                <Column> {
                    children: [
                        <Text> {
                            style: TextStyle {
                                font: Font::from_family("Arial"),

                                size: 16.0,
                                color: Color::from_rgb((0.0, 0.0, 0.0)),

                                h_align: HorizontalAlign::default(),
                                v_align: VerticalAlign::default(),
                            },

                            text: "foo".into(),
                        }.into(),
                        <Text> {
                            style: TextStyle {
                                font: Font::from_family("Arial"),

                                size: 16.0,
                                color: Color::from_rgb((0.0, 0.0, 0.0)),

                                h_align: HorizontalAlign::default(),
                                v_align: VerticalAlign::default(),
                            },

                            text: "bar".into(),
                        }.into(),
                    ],
                }
            },
        )
        .expect("failed to inflate widget");

        assert_eq!(
            tree.iter()
                .with_text("foo")
                .and_downcast()
                .next()
                .expect("should have found a widget")
                .text,
            "foo",
            "should have found the \"foo\" text widget"
        );

        assert_eq!(
            tree.iter()
                .with_text("bar")
                .and_downcast()
                .next()
                .expect("should have found a widget")
                .text,
            "bar",
            "should have found the \"bar\" text widget"
        );
    }
}

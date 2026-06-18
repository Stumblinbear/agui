use std::ops::Range;

use crate::{text::TextStyle, widget::Widget};

/// A piece of an [`InlineSpan`] tree: either styled [`Text`](InlineSpan::Text) or an inline
/// [`Widget`](InlineSpan::Widget).
#[allow(clippy::large_enum_variant)]
pub enum InlineSpan<T = ()> {
    /// Styled text and any nested spans beneath it.
    Text(TextSpan<T>),
    /// A widget laid out inline with the surrounding text.
    Widget(T),
}

/// A run of text with a [`style`](Self::style) that cascades into its [`children`](Self::children),
/// the building block of rich text.
///
/// The span's own [`text`](Self::text) is laid out first, then each child in order. A child's style
/// layers over this one, so a child sets only what it changes and inherits the rest.
pub struct TextSpan<Children = ()> {
    /// The text laid out before the children, in this span's style.
    pub text: String,

    /// The attributes applied to this span and inherited by its children.
    pub style: TextStyle,

    /// The spans laid out after [`text`](Self::text), inheriting this span's style.
    pub children: Vec<InlineSpan<Children>>,
}

impl<Children> Default for TextSpan<Children> {
    fn default() -> Self {
        Self {
            text: String::new(),
            style: TextStyle::default(),
            children: Vec::new(),
        }
    }
}

impl TextSpan<()> {
    /// Appends several child spans laid out after this span's text.
    #[must_use]
    pub fn children<Children>(
        self,
        children: impl IntoIterator<Item = InlineSpan<Children>>,
    ) -> TextSpan<Children> {
        TextSpan {
            text: self.text,
            style: self.style,
            children: children.into_iter().collect(),
        }
    }
}

impl<Children> TextSpan<Children> {
    /// Builds a span of `text` with no style of its own.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }

    /// Resolves the tree into a [`ParagraphContent`] and the inline widgets it references, in the
    /// order their placeholders appear in the text.
    pub fn flatten(self) -> (ParagraphContent, Vec<Children>) {
        let mut content = ParagraphContent::default();
        let mut widgets = Vec::new();

        flatten_span(
            InlineSpan::Text(self),
            &TextStyle::default(),
            &mut content,
            &mut widgets,
        );

        (content, widgets)
    }
}

impl From<&str> for TextSpan<()> {
    fn from(text: &str) -> Self {
        TextSpan::new(text)
    }
}

impl From<String> for TextSpan<()> {
    fn from(text: String) -> Self {
        TextSpan::new(text)
    }
}

impl<Children> From<&str> for InlineSpan<Children> {
    fn from(text: &str) -> Self {
        InlineSpan::Text(TextSpan {
            text: text.to_string(),
            style: TextStyle::default(),
            children: Vec::new(),
        })
    }
}

impl<Children> From<String> for InlineSpan<Children> {
    fn from(text: String) -> Self {
        InlineSpan::Text(TextSpan {
            text,
            style: TextStyle::default(),
            children: Vec::new(),
        })
    }
}

impl<Children> From<TextSpan<Children>> for InlineSpan<Children> {
    fn from(span: TextSpan<Children>) -> Self {
        InlineSpan::Text(span)
    }
}

impl<T> From<T> for InlineSpan<T>
where
    T: Widget,
{
    fn from(span: T) -> Self {
        InlineSpan::Widget(span)
    }
}

/// The widget-free description a [`RenderParagraph`](crate::text::RenderParagraph) shapes: the full
/// text, the absolute style of each run, and the byte offsets where inline widgets sit.
///
/// Runs partition the text into contiguous, non-overlapping ranges, each carrying the style that
/// applies there with the cascade already resolved.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ParagraphContent {
    /// The text of every span concatenated in layout order.
    pub text: String,
    /// The absolute style of each contiguous, non-overlapping run of the text.
    pub runs: Vec<(Range<usize>, TextStyle)>,
    /// The byte offset into [`text`](Self::text) of each inline widget, in order.
    pub placeholders: Vec<usize>,
}

impl From<&str> for ParagraphContent {
    fn from(text: &str) -> Self {
        TextSpan::<()>::new(text).flatten().0
    }
}

impl From<String> for ParagraphContent {
    fn from(text: String) -> Self {
        TextSpan::<()>::new(text).flatten().0
    }
}

fn flatten_span<Children>(
    span: InlineSpan<Children>,
    parent: &TextStyle,
    content: &mut ParagraphContent,
    widgets: &mut Vec<Children>,
) {
    match span {
        InlineSpan::Text(text_span) => {
            let effective = text_span.style.overlay(parent);

            if !text_span.text.is_empty() {
                let start = content.text.len();
                content.text.push_str(&text_span.text);
                let end = content.text.len();
                content.runs.push((start..end, effective.clone()));
            }

            for child in text_span.children {
                flatten_span(child, &effective, content, widgets);
            }
        }

        InlineSpan::Widget(widget) => {
            content.placeholders.push(content.text.len());
            widgets.push(widget);
        }
    }
}

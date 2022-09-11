use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

use std::{borrow::Cow, ops::Range};

pub trait EditableText: PartialEq + Eq + Clone + Into<Cow<'static, str>> + Send + Sync {
    fn as_str(&self) -> &str;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;

    /// Add text to the location
    fn insert(&mut self, index: usize, text: impl Into<Cow<'static, str>>);

    /// Remove a range of text
    fn remove(&mut self, range: Range<usize>);

    /// Replace range with new text
    fn replace(&mut self, range: Range<usize>, txt: impl Into<Cow<'static, str>>);

    /// Get the next grapheme offset from the current offset if it exists
    fn next_grapheme_offset(&self, current: usize) -> Option<usize>;
    /// Get the previous grapheme offset from the current offset if it exists
    fn prev_grapheme_offset(&self, current: usize) -> Option<usize>;

    /// Get the next word offset from the current offset if it exists
    fn next_word_offset(&self, current: usize) -> Option<usize>;
    /// Get the prev word offset from the current offset if it exists
    fn prev_word_offset(&self, current: usize) -> Option<usize>;
}

impl EditableText for String {
    fn as_str(&self) -> &str {
        self.as_str()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn insert(&mut self, index: usize, txt: impl Into<Cow<'static, str>>) {
        self.insert_str(index, &txt.into());
    }

    fn remove(&mut self, range: Range<usize>) {
        self.replace_range(range, "");
    }

    fn replace(&mut self, range: Range<usize>, txt: impl Into<Cow<'static, str>>) {
        self.replace_range(range, &txt.into());
    }

    fn next_grapheme_offset(&self, from: usize) -> Option<usize> {
        let mut cursor = GraphemeCursor::new(from, self.len(), true);
        cursor.next_boundary(self, 0).unwrap()
    }

    fn prev_grapheme_offset(&self, from: usize) -> Option<usize> {
        let mut cursor = GraphemeCursor::new(from, self.len(), true);
        cursor.prev_boundary(self, 0).unwrap()
    }

    fn next_word_offset(&self, from: usize) -> Option<usize> {
        let mut offset = from;
        let mut passed_alphanumeric = false;

        for next_grapheme in self.get(from..)?.graphemes(true) {
            let is_alphanumeric = next_grapheme.chars().next()?.is_alphanumeric();

            if is_alphanumeric {
                passed_alphanumeric = true;
            } else if passed_alphanumeric {
                return Some(offset);
            }

            offset += next_grapheme.len();
        }

        Some(self.len())
    }

    fn prev_word_offset(&self, from: usize) -> Option<usize> {
        let mut offset = from;
        let mut passed_alphanumeric = false;

        for prev_grapheme in self.get(0..from)?.graphemes(true).rev() {
            let is_alphanumeric = prev_grapheme.chars().next()?.is_alphanumeric();

            if is_alphanumeric {
                passed_alphanumeric = true;
            } else if passed_alphanumeric {
                return Some(offset);
            }

            offset -= prev_grapheme.len();

            if offset == 0 {
                return Some(0);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::EditableText;

    #[test]
    fn next_grapheme_offset() {
        assert_eq!(
            Some(6),
            String::from("This is some text").next_grapheme_offset(5)
        );

        assert_eq!(
            Some(5),
            String::from("Thi§ is some text").next_grapheme_offset(3)
        );
    }

    #[test]
    fn prev_grapheme_offset() {
        assert_eq!(
            Some(4),
            String::from("This is some text").prev_grapheme_offset(5)
        );

        assert_eq!(
            Some(3),
            String::from("Thi§ is some text").prev_grapheme_offset(5)
        );
    }

    #[test]
    fn next_word_offset() {
        assert_eq!(
            Some(7),
            String::from("This is some text").next_word_offset(5)
        );
    }

    #[test]
    fn prev_word_offset() {
        assert_eq!(
            Some(0),
            String::from("This is some text").prev_word_offset(5)
        );
    }
}

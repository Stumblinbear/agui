#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum TextBaseline {
    // The horizontal line used to align the bottom of glyphs for alphabetic characters.
    Alphabetic,

    // The horizontal line used to align ideographic characters.
    Ideographic,
}

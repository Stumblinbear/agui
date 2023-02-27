#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum TextDirection {
    #[default]
    LeftToRight,
    RightToLeft,
}

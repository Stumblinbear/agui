#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClipBehavior {
    #[default]
    None,
    Hard,
    AntiAliased,
}

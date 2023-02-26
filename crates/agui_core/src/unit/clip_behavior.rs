#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ClipBehavior {
    #[default]
    None,
    Hard,
    AntiAliased,
}

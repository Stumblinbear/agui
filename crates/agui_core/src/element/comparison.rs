#[derive(Debug, PartialEq, Eq)]
pub enum ElementComparison {
    /// The widgets were of the same type and instance.
    Identical,

    /// The widgets was changed and the element must be rebuilt to reflect the changes.
    Changed,

    /// The widgets were not of the same type and a new element must be created.
    Invalid,
}

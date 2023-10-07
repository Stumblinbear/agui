#[derive(Debug, PartialEq, Eq)]
pub enum ElementUpdate {
    /// The element was updated, but no rebuild is necessary.
    Noop,

    /// The element was updated and a rebuild is necessary to capture the changes.
    RebuildNecessary,

    /// The widgets were not of the same type and a new element must be created.
    Invalid,
}

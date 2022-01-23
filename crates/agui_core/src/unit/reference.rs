use std::{fmt::Debug, sync::Arc};

/// Holds a reference that is either `None` or `Some`.
///
/// It's used to give a single field that can accept `Option` (without additional wrapping),
/// an owned value, or a reference to an owned value (to prevent unnecessary clones).
///
/// # Example
///
/// ```ignore
/// pub struct Button {
///     // Allows the Button to provide its own default, accept an owned value from a parent
///     // widget, or a reference to a layout.
///     pub layout: Ref<Layout>,
/// }
/// ```
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Ref<V>
where
    V: ?Sized,
{
    /// No value.
    None,

    /// Has value.
    Some(Arc<V>),
}

impl<V> Default for Ref<V> {
    fn default() -> Self {
        Self::None
    }
}

impl<V> Ref<V> {
    /// Creates an Owned reference to `value`.
    pub fn new(value: V) -> Self {
        Self::Some(Arc::new(value))
    }

    /// Returns false if this reference points to nothing
    pub fn is_none(&self) -> bool {
        match self {
            Self::None => true,
            Self::Some(_) => false,
        }
    }

    /// Returns true if this reference points to a value in memory.
    pub fn is_some(&self) -> bool {
        match self {
            Self::None => false,
            Self::Some(_) => true,
        }
    }

    /// Attempts to fetch the value that this reference is wrapping.
    pub fn try_get(&self) -> Option<Arc<V>> {
        match self {
            Self::None => None,
            Self::Some(value) => Some(Arc::clone(value)),
        }
    }

    /// Fetch the underlying value that this reference is wrapping.
    ///
    /// # Panics
    ///
    /// Will panic if the value no longer exists, or the reference is empty.
    pub fn get(&self) -> Arc<V> {
        match self {
            Self::None => panic!("layout ref points to nothing"),
            Self::Some(value) => Arc::clone(value),
        }
    }
}

impl<V> Clone for Ref<V> {
    fn clone(&self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Some(value) => Self::Some(Arc::clone(value)),
        }
    }
}

impl<T> From<T> for Ref<T> {
    fn from(val: T) -> Self {
        Self::Some(Arc::new(val))
    }
}

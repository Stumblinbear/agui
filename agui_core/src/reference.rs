use std::{
    fmt::Debug,
    rc::{Rc, Weak},
};

/// Holds a reference that is either `None`, `Owned`, or `Borrowed`.
///
/// It's used to give a single field that can accept `Option` (without additional wrapping),
/// an owned value, or a reference to an owned value (to prevent unnecessary clones).
///
/// # Example
/// 
/// ```
/// pub struct Button {
///     // Allows the Button to provide its own default, accept an owned value from a parent
///     // widget, or a reference to a layout.
///     pub layout: Ref<Layout>,
/// }
/// ```
pub enum Ref<V> {
    /// No value.
    None,

    /// Owned data.
    Owned(Rc<V>),

    /// Borrowed data. Unlike Owned, this is not guaranteed to exist, as it only
    /// holds a weak reference to the Owned value. It should never be expected for the value
    /// to exist.
    Borrowed(Weak<V>),
}

impl<V> Default for Ref<V> {
    fn default() -> Self {
        Self::None
    }
}

impl<V> Debug for Ref<V>
where
    V: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Owned(value) => Debug::fmt(&value, f),
            Self::Borrowed(value) => match value.upgrade() {
                Some(value) => Debug::fmt(&value, f),
                None => write!(f, "Gone"),
            },
        }
    }
}

impl<V> Ref<V> {
    /// Creates an Owned reference to `value`.
    #[must_use]
    pub fn new(value: V) -> Self {
        Self::Owned(Rc::new(value))
    }

    /// Returns true if this reference points to a value in memory.
    /// 
    /// Will always be `true` if the Ref is `Owned`, will always be `None` if the Ref is `None`,
    /// but if it's `Borrowed`, it will only return `true` if the borrowed value is still in memory.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        match self {
            Self::None => false,
            Self::Owned(_) => true,
            Self::Borrowed(weak) => weak.strong_count() > 0,
        }
    }

    /// Attempst to fetch the value that this reference is wrapping.
    #[must_use]
    pub fn try_get(&self) -> Option<Rc<V>> {
        match self {
            Self::None => None,
            Self::Owned(value) => Some(Rc::clone(value)),
            Self::Borrowed(weak) => weak.upgrade(),
        }
    }

    /// Fetch the underlying value that this reference is wrapping.
    /// 
    /// # Panics
    ///
    /// Will panic if the value no longer exists, or the reference is empty.
    #[must_use]
    pub fn get(&self) -> Rc<V> {
        match self {
            Self::None => panic!("layout ref points to nothing"),
            Self::Owned(value) => Rc::clone(value),
            Self::Borrowed(weak) => match weak.upgrade() {
                Some(value) => value,
                None => panic!("value no longer exists"),
            },
        }
    }
}

impl<V> Clone for Ref<V> {
    fn clone(&self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Owned(value) => Self::Borrowed(Rc::downgrade(value)),
            Self::Borrowed(value) => Self::Borrowed(Weak::clone(value)),
        }
    }
}

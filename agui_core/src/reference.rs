use std::{
    fmt::Debug,
    rc::{Rc, Weak},
};

pub enum Ref<V> {
    None,
    Owned(Rc<V>),
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
    #[must_use]
    pub fn new(value: V) -> Self {
        Self::Owned(Rc::new(value))
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        match self {
            Self::None => false,
            Self::Owned(_) | Self::Borrowed(_) => true,
        }
    }

    #[must_use]
    pub fn try_get(&self) -> Option<Rc<V>> {
        match self {
            Self::None => None,
            Self::Owned(value) => Some(Rc::clone(value)),
            Self::Borrowed(weak) => weak.upgrade(),
        }
    }

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

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Unique(u64),
    Local(u64),
    Global(u64),
}

impl Key {
    #[must_use]
    pub const fn single() -> Self {
        Self::Local(0)
    }

    #[must_use]
    pub fn unique() -> Self {
        Self::Unique(rand::thread_rng().gen())
    }

    #[must_use]
    pub fn local<V>(value: V) -> Self
    where
        V: Hash,
    {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        Self::Local(hasher.finish())
    }

    #[must_use]
    pub fn global<V>(value: V) -> Self
    where
        V: Hash,
    {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        Self::Global(hasher.finish())
    }
}

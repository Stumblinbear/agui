use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use rand::Rng;

/// The key used for caching a widget.
///
/// This is used as a cache key for retaining a widget across rebuilds. The `u64` within it must be
/// unique within the scope indicated by the variant, or a `panic!` will occur.
///
/// In order for a widget to be persisted across rebuilds, it must be destroyed and recreated in a single
/// frame, it cannot be delayed or the old state will be lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    /// A local key.
    ///
    /// This should be unique within a widget's `build()` method. Any widget with the same key across
    /// rebuilds will be cached and re-parented, instead of rebuilt.
    Local(u64),

    /// A global key.
    ///
    /// This should be unique throughout the entire system.
    Global(u64),
}

impl Key {
    /// A helper function to cache a single `Local` widget.
    pub const fn single() -> Self {
        Self::Local(0)
    }

    /// Create a `Local` key based on the `value`.
    pub fn local<V>(value: V) -> Self
    where
        V: Hash,
    {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        Self::Local(hasher.finish())
    }

    /// Create a `Global` key based on the `value`.
    pub fn global<V>(value: V) -> Self
    where
        V: Hash,
    {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        Self::Global(hasher.finish())
    }

    /// Creates a `Unique` key.
    ///
    /// This should generally be created "above" the widget that uses it as a key, because it's
    /// designed to be created anew on each rebuild.
    pub fn unique() -> Self {
        Self::Global(rand::thread_rng().gen())
    }
}

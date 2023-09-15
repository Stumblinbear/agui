use std::hash::{Hash, Hasher};

use rand::Rng;
use rustc_hash::FxHasher;

/// The key used for caching a widget.
///
/// This is used as a cache key for retaining a widget across rebuilds. The `u64` within it must be
/// unique within the scope indicated by the variant, or a `panic!` will occur.
///
/// In order for a widget to be persisted across rebuilds, it must be destroyed and recreated in a single
/// frame, it cannot be delayed or the old state will be lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        let mut hasher = FxHasher::default();
        value.hash(&mut hasher);
        Self::Local(hasher.finish())
    }

    /// Create a `Global` key based on the `value`.
    pub fn global<V>(value: V) -> Self
    where
        V: Hash,
    {
        let mut hasher = FxHasher::default();
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

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Local(hash) | Key::Global(hash) => hash.fmt(f),
        }
    }
}

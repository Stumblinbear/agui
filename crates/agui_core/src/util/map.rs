use std::{any::TypeId, hash::BuildHasherDefault};

use super::hasher::TypeIdHasher;

#[allow(clippy::disallowed_types)]
pub type TypeMap<V> = std::collections::HashMap<TypeId, V, BuildHasherDefault<TypeIdHasher>>;

#[allow(clippy::disallowed_types)]
pub type TypeSet = std::collections::HashSet<TypeId, BuildHasherDefault<TypeIdHasher>>;

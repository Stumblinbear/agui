use std::{any::TypeId, hash::BuildHasherDefault};

use super::hasher::TypeIdHasher;

#[allow(clippy::disallowed_types)]
pub type TypeIdMap<V> = std::collections::HashMap<TypeId, V, BuildHasherDefault<TypeIdHasher>>;

#[allow(clippy::disallowed_types)]
pub type TypeIdSet = std::collections::HashSet<TypeId, BuildHasherDefault<TypeIdHasher>>;

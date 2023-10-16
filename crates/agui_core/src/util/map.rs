use std::{any::TypeId, hash::BuildHasherDefault};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::element::ElementId;

use super::hasher::TypeIdHasher;

#[allow(clippy::disallowed_types)]
pub type TypeMap<V> = std::collections::HashMap<TypeId, V, BuildHasherDefault<TypeIdHasher>>;

#[allow(clippy::disallowed_types)]
pub type TypeSet = std::collections::HashSet<TypeId, BuildHasherDefault<TypeIdHasher>>;

pub type ElementMap<V> = FxHashMap<ElementId, V>;
pub type ElementSet = FxHashSet<ElementId>;

use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    hash::BuildHasherDefault,
};

use super::hasher::TypeIdHasher;

pub type TypeMap<V> = HashMap<TypeId, V, BuildHasherDefault<TypeIdHasher>>;
pub type TypeSet = HashSet<TypeId, BuildHasherDefault<TypeIdHasher>>;

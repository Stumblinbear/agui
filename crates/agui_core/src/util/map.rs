use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    hash::BuildHasherDefault,
};

use fnv::{FnvHashMap, FnvHashSet};

use crate::manager::{plugin::PluginId, widget::WidgetId};

use super::hasher::TypeIdHasher;

pub type TypeMap<V> = HashMap<TypeId, V, BuildHasherDefault<TypeIdHasher>>;
pub type TypeSet = HashSet<TypeId, BuildHasherDefault<TypeIdHasher>>;

pub type WidgetMap<V> = FnvHashMap<WidgetId, V>;
pub type WidgetSet = FnvHashSet<WidgetId>;

pub type PluginMap<V> = HashMap<PluginId, V, BuildHasherDefault<TypeIdHasher>>;
pub type PluginSet = HashSet<PluginId, BuildHasherDefault<TypeIdHasher>>;

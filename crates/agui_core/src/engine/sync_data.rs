use std::{hash::BuildHasherDefault, sync::Arc};

use rustc_hash::FxHasher;
use slotmap::SparseSecondaryMap;

use crate::{
    element::{deferred::resolver::DeferredResolver, Element, ElementId},
    util::tree::Tree,
};

pub struct SyncTreeData<'sync> {
    pub(crate) element_tree: &'sync Tree<ElementId, Element>,

    pub(crate) deferred_resolvers: &'sync SparseSecondaryMap<
        ElementId,
        Arc<dyn DeferredResolver>,
        BuildHasherDefault<FxHasher>,
    >,
}

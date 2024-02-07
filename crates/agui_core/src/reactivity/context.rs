use std::collections::VecDeque;

use crate::util::tree::Tree;

pub struct ReactiveTreeMountContext<'ctx, K, V>
where
    K: slotmap::Key,
{
    pub tree: &'ctx Tree<K, V>,

    pub parent_id: &'ctx Option<K>,
    pub node_id: &'ctx K,
}

pub struct ReactiveTreeBuildContext<'ctx, K, V>
where
    K: slotmap::Key,
{
    pub tree: &'ctx Tree<K, V>,

    pub node_id: &'ctx K,
    pub value: &'ctx mut V,

    pub build_queue: &'ctx mut VecDeque<K>,
}

pub struct ReactiveTreeUnmountContext<'ctx, K, V>
where
    K: slotmap::Key,
{
    pub tree: &'ctx Tree<K, V>,

    pub node_id: &'ctx K,
}

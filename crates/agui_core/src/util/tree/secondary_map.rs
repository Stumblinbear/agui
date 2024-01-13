use std::{
    collections::VecDeque,
    ops::{Index, IndexMut},
};

use slotmap::{
    sparse_secondary::{Iter, IterMut},
    SparseSecondaryMap,
};

use crate::util::tree::{
    iter::{DownwardIterator, ParentIterator, SubtreeIterator, UpwardIterator},
    node::TreeNode,
};

pub struct SecondaryTreeMap<K, V>
where
    K: slotmap::Key,
{
    nodes: SparseSecondaryMap<K, TreeNode<K, V>>,
}

impl<K, V> Default for SecondaryTreeMap<K, V>
where
    K: slotmap::Key,
{
    fn default() -> Self {
        Self {
            nodes: SparseSecondaryMap::default(),
        }
    }
}

impl<K, V> SecondaryTreeMap<K, V>
where
    K: slotmap::Key,
{
    pub fn contains(&self, node_id: K) -> bool {
        self.nodes.contains_key(node_id)
    }

    pub fn get_node(&self, node_id: K) -> Option<&TreeNode<K, V>> {
        self.nodes.get(node_id)
    }

    pub fn get_node_mut(&mut self, node_id: K) -> Option<&mut TreeNode<K, V>> {
        self.nodes.get_mut(node_id)
    }

    pub fn get(&self, node_id: K) -> Option<&V> {
        self.get_node(node_id).map(|node| node.value())
    }

    pub fn get_mut(&mut self, node_id: K) -> Option<&mut V> {
        self.get_node_mut(node_id).map(|node| node.value_mut())
    }

    pub fn get_parent(&self, node_id: K) -> Option<K> {
        self.get_node(node_id).and_then(|node| node.parent())
    }

    pub fn get_child(&self, node_id: K, idx: usize) -> Option<K> {
        self.get_node(node_id)
            .and_then(|node| node.children.get(idx).copied())
    }

    pub fn get_children(&self, node_id: K) -> Option<&Vec<K>> {
        self.get_node(node_id).map(|node| &node.children)
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn get_depth(&self, node_id: K) -> Option<usize> {
        self.nodes.get(node_id).map(|node| node.depth)
    }

    pub(super) fn add(&mut self, parent_id: Option<K>, id: K, value: V) -> K {
        let node_id = self.nodes.insert(
            id,
            TreeNode {
                depth: 0,
                value: Some(value),
                parent: parent_id,
                children: Vec::new(),
            },
        );

        self.propagate_node(parent_id, node_id);

        node_id
    }

    pub(super) fn remove(&mut self, node_id: K) -> Option<V> {
        if let Some(mut node) = self.nodes.remove(node_id) {
            if let Some(parent_id) = node.parent {
                if let Some(parent) = self.nodes.get_mut(parent_id) {
                    // Remove the child from its parent
                    parent.children.remove(
                        parent
                            .children
                            .iter()
                            .position(|child_id| node_id == *child_id)
                            .expect("unable to find child in removed node's parent"),
                    );
                }
            }

            Some(node.value.take().expect("node is currently in use"))
        } else {
            None
        }
    }

    /// Retains only the children of the given node specified by the predicate.
    pub fn retain_children<F>(&mut self, node_id: K, mut func: F)
    where
        F: FnMut(&K) -> bool,
    {
        let Some(node) = self.get_node_mut(node_id) else {
            return;
        };

        let mut children = Vec::new();

        // With many children, it's potentially very expensive to allocate a new
        // array that would contian all of the children that need to be removed.
        // Instead we create a new array, which will not allocate, and swap it
        // with the node's current children. This lets us `retain` the children
        // while also modifying the tree in-place.
        std::mem::swap(&mut children, &mut node.children);

        children.retain(|child_id| {
            if !func(child_id) {
                // We don't need to call `self.remove` here, because we've already
                // removed the child from the parent's children list.
                self.nodes.remove(*child_id);

                false
            } else {
                true
            }
        });

        let node = self
            .get_node_mut(node_id)
            .expect("node was removed while retaining children");

        std::mem::swap(&mut children, &mut node.children);
    }

    pub(super) fn clear(&mut self) {
        self.nodes.clear();
    }

    /// Moves a node from one parent to another.
    ///
    /// Returns `true` if the node was moved, `false` if the node was already a child of the new parent.
    pub(super) fn reparent(&mut self, new_parent_id: Option<K>, node_id: K) -> bool {
        if let Some(node) = self.nodes.get(node_id) {
            if let Some(parent_id) = node.parent {
                if let Some(parent) = self.nodes.get_mut(parent_id) {
                    let child_idx = parent
                        .children
                        .iter()
                        .position(|child_id| node_id == *child_id)
                        .expect("unable to find child in removed node's parent");

                    // If the node isn't being moved to an entirely new parent
                    if Some(parent_id) == new_parent_id {
                        // If the widget is already the last child in the parent, don't do anything
                        if child_idx == parent.children.len() - 1 {
                            return false;
                        }

                        parent.children.remove(child_idx);

                        parent.children.push(node_id);

                        return false;
                    } else {
                        // Remove the child from its parent
                        parent.children.remove(child_idx);
                    }
                }
            }

            self.propagate_node(new_parent_id, node_id);
        }

        true
    }

    fn propagate_node(&mut self, parent_id: Option<K>, node_id: K) {
        let mut new_depth = 0;

        if let Some(parent_id) = parent_id {
            if let Some(parent) = self.nodes.get_mut(parent_id) {
                new_depth = parent.depth + 1;

                parent.children.push(node_id);
            } else {
                panic!("cannot add a node to a parent that doesn't exist");
            }
        }

        let node = &mut self.nodes[node_id];

        node.parent = parent_id;

        if node.depth != new_depth {
            let diff: i32 = (new_depth as i32) - (node.depth as i32);

            node.depth = new_depth;

            // If the node had children, propagate the depth difference
            if !node.children.is_empty() {
                let mut queue = VecDeque::from(node.children.clone());

                while let Some(child_id) = queue.pop_front() {
                    let child = self
                        .nodes
                        .get_mut(child_id)
                        .expect("unable to update child's depth, as it's not in the tree");

                    child.depth = ((child.depth as i32) + diff) as usize;

                    queue.extend(child.children.iter());
                }
            }
        }
    }

    pub(super) fn take(&mut self, node_id: K) -> Option<V> {
        self.nodes
            .get_mut(node_id)
            .map(|node| node.value.take().expect("node is currently in use"))
    }

    pub(super) fn replace(&mut self, node_id: K, value: V) {
        self.nodes
            .get_mut(node_id)
            .map(|node| node.value.replace(value));
    }

    pub fn iter(&self) -> Iter<K, TreeNode<K, V>> {
        self.nodes.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<K, TreeNode<K, V>> {
        self.nodes.iter_mut()
    }
}

impl<K, V> Index<K> for SecondaryTreeMap<K, V>
where
    K: slotmap::Key,
{
    type Output = V;

    fn index(&self, key: K) -> &Self::Output {
        self.nodes[key]
            .value
            .as_ref()
            .expect("node is currently in use")
    }
}

impl<K, V> IndexMut<K> for SecondaryTreeMap<K, V>
where
    K: slotmap::Key,
{
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        self.nodes[key]
            .value
            .as_mut()
            .expect("node is currently in use")
    }
}

use std::collections::VecDeque;

use crate::util::tree::{
    errors::{NodeInUse, ReorderChildrenError, ReparentError, SwapSiblingsError},
    iter::{
        ChildIterator, DownwardIter, Iter, IterableTree, ParentIterator, SubtreeIterator,
        UpwardIterator,
    },
    node::TreeNode,
    storage::{
        sealed::{TreeContainer, TreeContainerAdd, TreeContainerInsert},
        HopSlotMapStorage, TreeStorage,
    },
};

pub struct Tree<K, V, Storage = HopSlotMapStorage>
where
    K: slotmap::Key,
    Storage: TreeStorage,
{
    root: Option<K>,

    nodes: Storage::Container<K, TreeNode<K, V>>,
}

impl<K, V, Storage> Default for Tree<K, V, Storage>
where
    K: slotmap::Key,
    Storage: TreeStorage,
    Storage::Container<K, TreeNode<K, V>>: Default,
{
    fn default() -> Self {
        Self {
            root: None,

            nodes: Storage::Container::default(),
        }
    }
}

impl<K, V, Storage> Tree<K, V, Storage>
where
    K: slotmap::Key,
    Storage: TreeStorage,
{
    pub fn root(&self) -> Option<K> {
        self.root
    }

    /// Returns [`true`] if the tree contains `node_id`.
    pub fn contains(&self, node_id: K) -> bool {
        self.nodes.contains_key(node_id)
    }

    pub fn get_node(&self, node_id: K) -> Option<&TreeNode<K, V>> {
        self.nodes.get(node_id)
    }

    pub fn get_node_mut(&mut self, node_id: K) -> Option<&mut TreeNode<K, V>> {
        self.nodes.get_mut(node_id)
    }

    /// Returns a reference to the value with the given ID.
    ///
    /// # Panics
    ///
    /// If the node is currently in use (i.e. it has been pulled from the tree
    /// via [`Tree::with`]) then this method will panic.
    pub fn get(&self, node_id: K) -> Option<&V> {
        self.get_node(node_id).map(|node| node.borrow())
    }

    /// Returns a mutable reference to the value with the given ID.
    ///
    /// # Panics
    ///
    /// If the node is currently in use (i.e. it has been pulled from the tree
    /// via [`Tree::with`]) then this method will panic.
    pub fn get_mut(&mut self, node_id: K) -> Option<&mut V> {
        self.get_node_mut(node_id).map(|node| node.borrow_mut())
    }

    pub fn get_parent(&self, node_id: K) -> Option<&K> {
        self.get_node(node_id).and_then(|node| node.parent())
    }

    pub fn get_child(&self, node_id: K, idx: usize) -> Option<K> {
        self.get_node(node_id)
            .and_then(|node| node.children.get(idx).copied())
    }

    pub fn get_children(&self, node_id: K) -> Option<&Vec<K>> {
        self.get_node(node_id).map(|node| &node.children)
    }

    /// Returns if the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the number of nodes in the tree.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn get_depth(&self, node_id: K) -> Option<usize> {
        self.nodes.get(node_id).map(|node| node.depth)
    }
}

impl<K, V, Storage> Tree<K, V, Storage>
where
    K: slotmap::Key,
    Storage: TreeStorage,
    Storage::Container<K, TreeNode<K, V>>: TreeContainerAdd<K, TreeNode<K, V>>,
{
    pub fn add(&mut self, parent_id: Option<K>, value: V) -> K {
        let node_id = self.nodes.add(TreeNode {
            depth: 0,
            value: Some(value),
            parent: parent_id,
            children: Vec::new(),
        });

        if parent_id.is_none() {
            self.root = Some(node_id);
        }

        self.propagate_node(parent_id, node_id);

        node_id
    }

    pub fn add_with_key<F>(&mut self, parent_id: Option<K>, f: F) -> K
    where
        F: FnOnce(&mut Self, K) -> V,
    {
        let node_id = self.nodes.add(TreeNode {
            depth: 0,
            value: None,
            parent: parent_id,
            children: Vec::new(),
        });

        self.propagate_node(parent_id, node_id);

        let value = f(self, node_id);

        self.nodes
            .get_mut(node_id)
            .expect("node was removed from the tree before it was fully initialized")
            .value
            .replace(value);

        if parent_id.is_none() {
            self.root = Some(node_id);
        }

        node_id
    }
}

impl<K, V, Storage> Tree<K, V, Storage>
where
    K: slotmap::Key,
    Storage: TreeStorage,
    Storage::Container<K, TreeNode<K, V>>: TreeContainerInsert<K, TreeNode<K, V>>,
{
    pub fn insert(&mut self, parent_id: Option<K>, node_id: K, value: V) -> K {
        self.nodes.insert(
            node_id,
            TreeNode {
                depth: 0,
                value: Some(value),
                parent: parent_id,
                children: Vec::new(),
            },
        );

        if parent_id.is_none() {
            self.root = Some(node_id);
        }

        self.propagate_node(parent_id, node_id);

        node_id
    }
}

impl<K, V, Storage> Tree<K, V, Storage>
where
    K: slotmap::Key,
    Storage: TreeStorage,
{
    pub fn remove_subtree(&mut self, node_id: K) {
        let Some(children) = self.get_children(node_id) else {
            return;
        };

        let mut remove_queue = VecDeque::with_capacity(children.len().max(8));

        remove_queue.extend(children);

        // We only have to normally remove the first node, since we're about
        // to completely obliterate the rest of its subtree there's no reason
        // to bother doing cleanup on the rest of the descendant nodes.
        let _ = self.remove(node_id);

        while let Some(node_id) = remove_queue.pop_front() {
            if let Some(node) = self.nodes.remove(node_id) {
                remove_queue.extend(node.children);
            }
        }
    }

    pub fn remove_node(&mut self, node_id: K) -> Option<TreeNode<K, V>> {
        if self.root == Some(node_id) {
            self.root = None;
        }

        if let Some(node) = self.nodes.remove(node_id) {
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

            Some(node)
        } else {
            None
        }
    }

    pub fn remove(&mut self, node_id: K) -> Result<Option<V>, NodeInUse> {
        if let Some(mut node) = self.remove_node(node_id) {
            Ok(Some(node.take()?))
        } else {
            Ok(None)
        }
    }

    pub fn retain_subtree<F>(&mut self, node_id: K, mut func: F)
    where
        F: FnMut(&K) -> bool,
    {
        if !self.contains(node_id) {
            return;
        }

        let mut remove_queue = VecDeque::with_capacity(8);

        remove_queue.push_back(node_id);

        while let Some(node_id) = remove_queue.pop_front() {
            for child_id in self.get_children(node_id).unwrap() {
                if !func(child_id) {
                    remove_queue.push_back(*child_id);
                }
            }

            if !func(&node_id) {
                let _ = self.remove(node_id);
            }
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

    pub fn clear(&mut self) {
        self.root = None;
        self.nodes.clear();
    }
}

impl<K, V, Storage> Tree<K, V, Storage>
where
    K: slotmap::Key,
    Storage: TreeStorage,
{
    /// Swaps two siblings that are children of the same parent.
    pub fn swap_siblings(
        &mut self,
        parent_id: K,
        sibling: ChildNode<K>,
        other_sibling: ChildNode<K>,
    ) -> Result<(), SwapSiblingsError> {
        let parent = self
            .nodes
            .get_mut(parent_id)
            .ok_or(SwapSiblingsError::ParentNotFound)?;

        if parent.children.is_empty() {
            return Err(SwapSiblingsError::SiblingNotFound);
        }

        let mut sibling_idx = match sibling {
            ChildNode::First => Some(0),
            ChildNode::Last => Some(parent.children.len() - 1),
            ChildNode::Index(idx) => Some(idx),
            ChildNode::Id(_) => None,
        };

        let mut other_sibling_idx = match other_sibling {
            ChildNode::First => Some(0),
            ChildNode::Last => Some(parent.children.len() - 1),
            ChildNode::Index(idx) => Some(idx),
            ChildNode::Id(_) => None,
        };

        if sibling_idx.is_none() || other_sibling_idx.is_none() {
            // If we found one of the siblings already, we check to see if the other sibling is
            // in the same position as the sibling we found. If it is, we can bail early.
            if let ChildNode::Id(other_sibling_id) = other_sibling {
                if let Some(sibling_idx) = sibling_idx {
                    if parent.children.get(sibling_idx) == Some(&other_sibling_id) {
                        return Ok(());
                    }
                }
            } else if let ChildNode::Id(sibling_id) = sibling {
                if let Some(other_sibling_idx) = other_sibling_idx {
                    if parent.children.get(other_sibling_idx) == Some(&sibling_id) {
                        return Ok(());
                    }
                }
            }

            for (i, child_id) in parent.children.iter().enumerate() {
                if sibling == ChildNode::Id(*child_id) {
                    sibling_idx = Some(i);

                    if other_sibling_idx.is_some() {
                        break;
                    }
                }

                if other_sibling == ChildNode::Id(*child_id) {
                    other_sibling_idx = Some(i);

                    if sibling_idx.is_some() {
                        break;
                    }
                }
            }
        }

        if let (Some(sibling_idx), Some(other_sibling_idx)) = (sibling_idx, other_sibling_idx) {
            parent.children.swap(sibling_idx, other_sibling_idx);

            Ok(())
        } else {
            Err(SwapSiblingsError::SiblingNotFound)
        }
    }

    /// Reorder all children of a node. The new children must contain all children of the parent,
    /// and cannot contain any additional children.
    pub fn reorder_children(
        &mut self,
        node_id: K,
        new_children: Vec<K>,
    ) -> Result<(), ReorderChildrenError> {
        let Some(node) = self.nodes.get_mut(node_id) else {
            return Err(ReorderChildrenError::NotFound);
        };

        if new_children.len() != node.children.len() {
            return Err(ReorderChildrenError::DisjointChildren);
        }

        // Ensure all children are present in the new children list
        // TODO: Switch to a hash lookup with large enough lengths?
        #[cfg(debug_assertions)]
        for child_id in &node.children {
            if !new_children.contains(child_id) {
                return Err(ReorderChildrenError::DisjointChildren);
            }
        }

        node.children = new_children;

        Ok(())
    }

    /// Moves a node from one parent to another.
    pub fn reparent(&mut self, new_parent_id: Option<K>, node_id: K) -> Result<(), ReparentError> {
        let Some(node) = self.nodes.get(node_id) else {
            return Err(ReparentError::NodeNotFound);
        };

        if new_parent_id.is_none() {
            self.root = Some(node_id);
        } else if self.root == Some(node_id) {
            self.root = None;
        }

        if let Some(parent_id) = node.parent {
            let Some(parent) = self.nodes.get_mut(parent_id) else {
                return Err(ReparentError::NewParentNotFound);
            };

            let child_idx = parent
                .children
                .iter()
                .position(|child_id| node_id == *child_id)
                .expect("unable to find child in removed node's parent");

            if Some(parent_id) == new_parent_id {
                return Err(ReparentError::Unmoved);
            }

            // Remove the child from its parent
            parent.children.remove(child_idx);

            self.propagate_node(new_parent_id, node_id);
        }

        Ok(())
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

        let node = self
            .nodes
            .get_mut(node_id)
            .expect("node missing while propagating");

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

    pub fn with<F, R>(&mut self, node_id: K, func: F) -> Option<R>
    where
        F: FnOnce(&mut Self, &mut V) -> R,
    {
        if let Some(mut value) = self.take(node_id) {
            let ret = func(self, &mut value);

            self.replace(node_id, value);

            Some(ret)
        } else {
            None
        }
    }

    pub fn take(&mut self, node_id: K) -> Option<V> {
        self.nodes
            .get_mut(node_id)
            .map(|node| node.value.take().expect("node is currently in use"))
    }

    pub fn replace(&mut self, node_id: K, value: V) {
        self.nodes
            .get_mut(node_id)
            .map(|node| node.value.replace(value));
    }

    // pub fn has_child(&self, node_id: K, child_id: K) -> bool {
    //     // Make sure they're actually in the tree
    //     if !self.nodes.contains_key(node_id) || !self.nodes.contains_key(child_id) {
    //         return false;
    //     }

    //     let child_depth = self.nodes[child_id].depth;

    //     // If the node's depth is below the child, it's impossible for the child to be in the parent
    //     if self.nodes[node_id].depth >= child_depth {
    //         return false;
    //     }

    //     for node_id in self.iter_down_from(node_id) {
    //         let node = self.nodes.get(node_id).expect("tree broken");

    //         // If we reach a depth lower than the child, bail, because the child won't be found. We do
    //         // not do an equality check, here, because we may find the child as a sibling
    //         if node.depth > child_depth {
    //             return false;
    //         }

    //         // The child exists under the parent
    //         if node_id == child_id {
    //             return true;
    //         }
    //     }

    //     false
    // }

    pub fn is_first_child(&self, node_id: K) -> bool {
        if let Some(parent_id) = self.get_parent(node_id) {
            if let Some(parent) = self.nodes.get(*parent_id) {
                return parent
                    .children
                    .first()
                    .map_or(true, |child_id| *child_id == node_id);
            }
        }

        true
    }

    pub fn is_last_child(&self, node_id: K) -> bool {
        if let Some(parent_id) = self.get_parent(node_id) {
            if let Some(parent) = self.nodes.get(*parent_id) {
                return parent
                    .children
                    .last()
                    .map_or(true, |child_id| *child_id == node_id);
            }
        }

        true
    }
}

impl<K, V, Storage> Tree<K, V, Storage>
where
    K: slotmap::Key,
    Storage: TreeStorage,
{
    pub fn iter(&self) -> Iter<K, V, Storage> {
        Iter::new(&self.nodes)
    }

    pub fn iter_down(&self) -> DownwardIter<K, Self> {
        DownwardIter {
            tree: self,
            node_id: self.root,
            first: true,
        }
    }

    pub fn iter_up(&self) -> UpwardIterator<K, Self> {
        UpwardIterator::from_root(self, self.root)
    }

    pub fn iter_children(&self, node_id: K) -> ChildIterator<K, Self> {
        ChildIterator {
            tree: self,
            node_id,
            current_child_id: None,
            first: true,
        }
    }

    pub fn iter_down_from(&self, node_id: K) -> DownwardIter<K, Self> {
        DownwardIter {
            tree: self,
            node_id: Some(node_id),
            first: true,
        }
    }

    pub fn iter_up_from(&self, node_id: K) -> UpwardIterator<K, Self> {
        UpwardIterator {
            tree: self,
            node_id: Some(node_id),
            first: true,
        }
    }

    pub fn iter_subtree<'a, F>(&'a self, node_id: K, filter: F) -> SubtreeIterator<'a, K, F, Self>
    where
        F: Fn(K) -> bool + 'a,
    {
        SubtreeIterator {
            tree: self,
            root_node_id: node_id,
            node_id: Some(node_id),
            first: true,
            filter,
        }
    }

    pub fn iter_parents(&self, node_id: K) -> ParentIterator<K, Self> {
        ParentIterator {
            tree: self,
            node_id: Some(node_id),
        }
    }
    // pub fn iter(&self) -> Iter<K, TreeNode<K, V>> {
    //     self.nodes.iter()
    // }

    // pub fn iter_mut(&mut self) -> IterMut<K, TreeNode<K, V>> {
    //     self.nodes.iter_mut()
    // }

    // /// Returns any nodes that do not have any other node as a parent.
    // pub fn filter_topmost<I>(&self, nodes: I) -> Vec<K>
    // where
    //     I: Iterator<Item = K>,
    // {
    //     let mut topmost = Vec::new();

    //     'main: for key in nodes {
    //         let tree_node = match self.nodes.get(key) {
    //             Some(widget) => widget,
    //             None => continue,
    //         };

    //         let node_depth = tree_node.depth;

    //         let mut i = 0;

    //         while i < topmost.len() {
    //             let (dirty_id, dirty_depth) = topmost[i];

    //             // If they're at the same depth, bail. No reason to check if they're children.
    //             if node_depth != dirty_depth {
    //                 if node_depth > dirty_depth {
    //                     // If the node is a child of one of the `topmost` nodes, bail
    //                     if self.has_child(dirty_id, key) {
    //                         continue 'main;
    //                     }
    //                 } else {
    //                     // If the node is a parent of a node already in the `topmost` vec, remove it
    //                     if self.has_child(key, dirty_id) {
    //                         topmost.remove(i);
    //                         continue;
    //                     }
    //                 }
    //             }

    //             i += 1;
    //         }

    //         topmost.push((key, node_depth));
    //     }

    //     topmost
    //         .into_iter()
    //         .map(|(widget_id, _)| widget_id)
    //         .collect::<Vec<_>>()
    // }
}

impl<K, V, Storage> IterableTree<K> for Tree<K, V, Storage>
where
    K: slotmap::Key,
    Storage: TreeStorage,
{
    type Value = V;

    fn get_node(&self, node_id: K) -> Option<&TreeNode<K, Self::Value>> {
        self.get_node(node_id)
    }
}

impl<K, V, Storage> std::fmt::Debug for Tree<K, V, Storage>
where
    K: slotmap::Key,
    V: std::fmt::Debug,
    Storage: TreeStorage,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct DebugNodes(usize);

        impl std::fmt::Debug for DebugNodes {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("[")?;

                f.write_fmt(format_args!(
                    "..{} item{}",
                    self.0,
                    if self.0 == 1 { "" } else { "s" }
                ))?;

                f.write_str("]")
            }
        }

        if f.alternate() {
            write!(f, "Tree")?;

            for node_id in self.iter_down() {
                writeln!(f)?;

                let depth = self.get_depth(node_id).unwrap();

                f.write_str("| ")?;

                f.write_str(&"    | ".repeat(depth / 3))?;

                if depth % 3 == 1 {
                    f.write_str("  ")?;
                } else if depth % 3 == 2 {
                    f.write_str("    ")?;
                }

                if let Some(value) = self.get(node_id) {
                    // We don't propagate the alternative formatting because otherwise the tree
                    // would be completely unreadable.
                    write!(f, "{:?}", value)?;
                } else {
                    f.write_str("In use")?;
                }

                write!(f, " ({:?})", node_id.data())?;
            }
        } else {
            f.debug_struct("Tree")
                .field("root", &self.root)
                .field("nodes", &DebugNodes(self.nodes.len()))
                .finish()?;
        }

        Ok(())
    }
}

impl<K, V, Storage> std::fmt::Display for Tree<K, V, Storage>
where
    K: slotmap::Key,
    V: std::fmt::Display,
    Storage: TreeStorage,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Tree")?;

        for node_id in self.iter_down() {
            let depth = self.get_depth(node_id).unwrap();

            f.write_str("  ")?;

            if depth > 0 {
                f.write_str(&"|     ".repeat(depth / 3))?;

                if depth % 3 == 1 {
                    f.write_str("| ")?;
                } else if depth % 3 == 2 {
                    f.write_str("|   ")?;
                }
            }

            if let Some(value) = self.get(node_id) {
                value.fmt(f)?;
            } else {
                f.write_str("Missing")?;
            }

            writeln!(f, " ({:?})", node_id.data())?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildNode<K> {
    First,
    Last,
    Index(usize),
    Id(K),
}

// impl<K, V, Storage> Index<K> for Tree<Storage>
// where
//     K: slotmap::Key,
//     Storage: TreeStorage<K, V>,
// {
//     type Output = V;

//     fn index(&self, key: K) -> &Self::Output {
//         self.nodes[key]
//             .value
//             .as_ref()
//             .expect("node is currently in use")
//     }
// }

// impl<K, V, Storage> IndexMut<K> for Tree<Storage>
// where
//     K: slotmap::Key,
//     Storage: TreeStorage<K, V>,
// {
//     fn index_mut(&mut self, key: K) -> &mut Self::Output {
//         self.nodes[key]
//             .value
//             .as_mut()
//             .expect("node is currently in use")
//     }
// }

#[cfg(test)]
mod tests {

    use crate::element::ElementId;

    use super::Tree;

    #[test]
    fn is_first_last_child() {
        let mut tree: Tree<ElementId, usize> = Tree::default();

        let root_id = tree.add(None, 0);

        assert!(
            tree.is_first_child(root_id),
            "root should return true for first child checks"
        );
        assert!(
            tree.is_last_child(root_id),
            "root should return true for last child checks"
        );
    }

    #[test]
    fn hierarchy() {
        let mut tree: Tree<ElementId, usize> = Tree::default();

        let root_id = tree.add(None, 0);

        let child_1 = tree.add(Some(root_id), 1);
        let child_1_1 = tree.add(Some(child_1), 2);
        let child_1_1_1 = tree.add(Some(child_1_1), 3);
        let child_1_2 = tree.add(Some(child_1), 4);
        let child_1_3 = tree.add(Some(child_1), 5);

        let child_2 = tree.add(Some(root_id), 6);

        let child_3 = tree.add(Some(root_id), 7);
        let child_3_1 = tree.add(Some(child_3), 8);

        assert!(
            tree.is_first_child(child_1),
            "child_1 is the first child of the parent"
        );
        assert!(
            !tree.is_last_child(child_1),
            "child_1 is not the last child of the parent"
        );

        assert!(
            tree.is_first_child(child_1_1),
            "child_1_1 is the first child of the parent"
        );
        assert!(
            !tree.is_last_child(child_1_1),
            "child_1_1 is not the last child of the parent"
        );

        assert!(
            tree.is_first_child(child_1_1_1),
            "child_1_1_1 is the first child of the parent"
        );
        assert!(
            tree.is_last_child(child_1_1_1),
            "child_1_1_1 is the last child of the parent"
        );

        assert!(
            !tree.is_first_child(child_1_2),
            "child_1_2 is not the first child of the parent"
        );
        assert!(
            !tree.is_last_child(child_1_2),
            "child_1_2 is not the last child of the parent"
        );

        assert!(
            !tree.is_first_child(child_1_3),
            "child_1_3 is not the first child of the parent"
        );
        assert!(
            tree.is_last_child(child_1_3),
            "child_1_3 is the last child of the parent"
        );

        assert!(
            !tree.is_first_child(child_2),
            "child_2 is not the first child of the parent"
        );
        assert!(
            !tree.is_last_child(child_2),
            "child_2 is not the last child of the parent"
        );

        assert!(
            !tree.is_first_child(child_3),
            "child_3 is not the first child of the parent"
        );
        assert!(
            tree.is_last_child(child_3),
            "child_3 is the last child of the parent"
        );

        assert!(
            tree.is_first_child(child_3_1),
            "child_3_1 is the first child of the parent"
        );
        assert!(
            tree.is_last_child(child_3_1),
            "child_3_1 is the last child of the parent"
        );
    }

    #[test]
    fn reparenting() {
        let mut tree: Tree<ElementId, usize> = Tree::default();

        let root_id = tree.add(None, 0);

        let child_1 = tree.add(Some(root_id), 1);
        let child_1_1 = tree.add(Some(child_1), 2);
        let child_1_1_1 = tree.add(Some(child_1_1), 3);

        let child_2 = tree.add(Some(root_id), 6);

        assert_eq!(
            tree.get_parent(child_1_1),
            Some(&child_1),
            "child_1_1 should be a child of child_1"
        );

        tree.reparent(Some(child_2), child_1_1)
            .expect("failed to reparent");

        assert_eq!(
            tree.get_parent(child_1_1),
            Some(&child_2),
            "child_1_1 should be a child of child_2"
        );

        assert_eq!(
            tree.get_parent(child_1_1_1),
            Some(&child_1_1),
            "child_1_1_1 should be a child of child_1_1"
        );
    }

    #[test]
    fn downward_iter() {
        let mut tree: Tree<ElementId, usize> = Tree::default();

        let root_id = tree.add(None, 0);

        let child_1 = tree.add(Some(root_id), 1);
        let child_1_1 = tree.add(Some(child_1), 2);
        let child_1_1_1 = tree.add(Some(child_1_1), 3);
        let child_1_2 = tree.add(Some(child_1), 4);
        let child_1_3 = tree.add(Some(child_1), 5);

        let child_2 = tree.add(Some(root_id), 6);

        let child_3 = tree.add(Some(root_id), 7);
        let child_3_1 = tree.add(Some(child_3), 8);

        let mut iter = tree.iter_down_from(root_id);

        assert_eq!(
            iter.next(),
            Some(root_id),
            "downward iterator's first element must be the root node"
        );
        assert_eq!(
            iter.next(),
            Some(child_1),
            "downward iterator should have returned child_1"
        );
        assert_eq!(
            iter.next(),
            Some(child_1_1),
            "downward iterator should have returned child_1_1"
        );
        assert_eq!(
            iter.next(),
            Some(child_1_1_1),
            "downward iterator should have returned child_1_1_1"
        );
        assert_eq!(
            iter.next(),
            Some(child_1_2),
            "downward iterator should have returned child_1_2"
        );
        assert_eq!(
            iter.next(),
            Some(child_1_3),
            "downward iterator should have returned child_1_3"
        );
        assert_eq!(
            iter.next(),
            Some(child_2),
            "downward iterator should have returned child_2"
        );
        assert_eq!(
            iter.next(),
            Some(child_3),
            "downward iterator should have returned child_3"
        );
        assert_eq!(
            iter.next(),
            Some(child_3_1),
            "downward iterator should have returned child_3_1"
        );
        assert_eq!(
            iter.next(),
            None,
            "downward iterator should have returned None"
        );
        assert_eq!(
            iter.next(),
            None,
            "downward iterator should have returned None"
        );

        let mut iter = tree.iter_down_from(child_2);

        assert_eq!(
            iter.next(),
            Some(child_2),
            "downward iterator should have returned child_2"
        );

        assert_eq!(
            iter.next(),
            Some(child_3),
            "downward iterator should have returned child_3"
        );
        assert_eq!(
            iter.next(),
            Some(child_3_1),
            "downward iterator should have returned child_3_1"
        );
        assert_eq!(
            iter.next(),
            None,
            "downward iterator should have returned None"
        );
        assert_eq!(
            iter.next(),
            None,
            "downward iterator should have returned None"
        );

        let mut iter = tree.iter_down_from(child_3);

        assert_eq!(
            iter.next(),
            Some(child_3),
            "downward iterator should have returned child_3"
        );
        assert_eq!(
            iter.next(),
            Some(child_3_1),
            "downward iterator should have returned child_3_1"
        );
        assert_eq!(
            iter.next(),
            None,
            "downward iterator should have returned None"
        );
    }

    #[test]
    fn upward_iter() {
        let mut tree: Tree<ElementId, usize> = Tree::default();

        let root_id = tree.add(None, 0);

        let child_1 = tree.add(Some(root_id), 1);
        let child_1_1 = tree.add(Some(child_1), 2);
        let child_1_1_1 = tree.add(Some(child_1_1), 3);
        let child_1_2 = tree.add(Some(child_1), 4);
        let child_1_3 = tree.add(Some(child_1), 5);

        let child_2 = tree.add(Some(root_id), 6);

        let child_3 = tree.add(Some(root_id), 7);
        let child_3_1 = tree.add(Some(child_3), 8);

        let mut iter = tree.iter_up_from(child_3_1);

        assert_eq!(
            iter.next(),
            Some(child_3_1),
            "upward iterator should have returned child_3_1"
        );
        assert_eq!(
            iter.next(),
            Some(child_3),
            "upward iterator should have returned child_3"
        );
        assert_eq!(
            iter.next(),
            Some(child_2),
            "upward iterator should have returned child_2"
        );
        assert_eq!(
            iter.next(),
            Some(child_1_3),
            "upward iterator should have returned child_1_3"
        );
        assert_eq!(
            iter.next(),
            Some(child_1_2),
            "upward iterator should have returned child_1_2"
        );
        assert_eq!(
            iter.next(),
            Some(child_1_1_1),
            "upward iterator should have returned child_1_1_1"
        );
        assert_eq!(
            iter.next(),
            Some(child_1_1),
            "upward iterator should have returned child_1_1"
        );
        assert_eq!(
            iter.next(),
            Some(child_1),
            "upward iterator should have returned child_1"
        );
        assert_eq!(
            iter.next(),
            Some(root_id),
            "upward iterator should have returned the root node"
        );
        assert_eq!(
            iter.next(),
            None,
            "upward iterator should have returned None"
        );
        assert_eq!(
            iter.next(),
            None,
            "upward iterator should have returned None"
        );

        let mut iter = tree.iter_up_from(child_1_2);

        assert_eq!(
            iter.next(),
            Some(child_1_2),
            "upward iterator should have returned child_1_2"
        );
        assert_eq!(
            iter.next(),
            Some(child_1_1_1),
            "upward iterator should have returned child_1_1_1"
        );
        assert_eq!(
            iter.next(),
            Some(child_1_1),
            "upward iterator should have returned child_1_1"
        );
        assert_eq!(
            iter.next(),
            Some(child_1),
            "upward iterator should have returned child_1"
        );
        assert_eq!(
            iter.next(),
            Some(root_id),
            "upward iterator should have returned the root node"
        );
        assert_eq!(
            iter.next(),
            None,
            "upward iterator should have returned None"
        );
    }

    #[test]
    fn subtree_iter() {
        let mut tree: Tree<ElementId, usize> = Tree::default();

        let root_id = tree.add(None, 0);

        let child_1 = tree.add(Some(root_id), 1);
        let child_1_1 = tree.add(Some(child_1), 2);
        let child_1_1_1 = tree.add(Some(child_1_1), 3);
        let child_1_2 = tree.add(Some(child_1), 4);
        let child_1_3 = tree.add(Some(child_1), 5);

        let child_2 = tree.add(Some(root_id), 6);

        let child_3 = tree.add(Some(root_id), 7);
        let child_3_1 = tree.add(Some(child_3), 8);

        let mut iter = tree.iter_subtree(child_1, |_| true);

        assert_eq!(
            iter.next(),
            Some(child_1),
            "subtree iterator's first element must be child_1"
        );
        assert_eq!(
            iter.next(),
            Some(child_1_1),
            "subtree iterator should have returned child_1_1"
        );
        assert_eq!(
            iter.next(),
            Some(child_1_1_1),
            "subtree iterator should have returned child_1_1_1"
        );
        assert_eq!(
            iter.next(),
            Some(child_1_2),
            "subtree iterator should have returned child_1_2"
        );
        assert_eq!(
            iter.next(),
            Some(child_1_3),
            "subtree iterator should have returned child_1_3"
        );
        assert_eq!(
            iter.next(),
            None,
            "subtree iterator should have returned None"
        );

        let mut iter = tree.iter_subtree(child_2, |_| true);

        assert_eq!(
            iter.next(),
            Some(child_2),
            "subtree iterator should have returned child_2"
        );
        assert_eq!(
            iter.next(),
            None,
            "subtree iterator should have returned None"
        );

        let mut iter = tree.iter_subtree(child_3, |_| true);

        assert_eq!(
            iter.next(),
            Some(child_3),
            "subtree iterator should have returned child_3"
        );
        assert_eq!(
            iter.next(),
            Some(child_3_1),
            "subtree iterator should have returned child_3_1"
        );
        assert_eq!(
            iter.next(),
            None,
            "subtree iterator should have returned None"
        );
    }

    #[test]
    fn depth_propagation() {
        let mut tree: Tree<ElementId, usize> = Tree::default();

        let root_id = tree.add(None, 0);

        let child_1 = tree.add(Some(root_id), 1);
        let child_1_1 = tree.add(Some(child_1), 2);
        let child_1_1_1 = tree.add(Some(child_1_1), 3);
        let child_1_2 = tree.add(Some(child_1), 4);
        let child_1_3 = tree.add(Some(child_1), 5);

        let child_2 = tree.add(Some(root_id), 6);

        let child_3 = tree.add(Some(root_id), 7);
        let child_3_1 = tree.add(Some(child_3), 8);

        assert_eq!(
            tree.get_depth(root_id),
            Some(0),
            "root node should have depth 0"
        );

        assert_eq!(
            tree.get_depth(child_1),
            Some(1),
            "child_1 should have depth 1"
        );
        assert_eq!(
            tree.get_depth(child_1_1),
            Some(2),
            "child_1_1 should have depth 2"
        );
        assert_eq!(
            tree.get_depth(child_1_1_1),
            Some(3),
            "child_1_1_1 should have depth 3"
        );
        assert_eq!(
            tree.get_depth(child_1_2),
            Some(2),
            "child_1_2 should have depth 2"
        );
        assert_eq!(
            tree.get_depth(child_1_3),
            Some(2),
            "child_1_3 should have depth 2"
        );

        assert_eq!(
            tree.get_depth(child_2),
            Some(1),
            "child_2 should have depth 1"
        );

        assert_eq!(
            tree.get_depth(child_3),
            Some(1),
            "child_3 should have depth 1"
        );
        assert_eq!(
            tree.get_depth(child_3_1),
            Some(2),
            "child_3_1 should have depth 2"
        );

        tree.reparent(Some(root_id), child_1_1)
            .expect("failed to reparent");

        assert_eq!(
            tree.get_depth(root_id),
            Some(0),
            "root node should have depth 0"
        );

        assert_eq!(
            tree.get_depth(child_1),
            Some(1),
            "child_1 should have depth 1"
        );
        assert_eq!(
            tree.get_depth(child_1_1),
            Some(1),
            "child_1_1 should have depth 1"
        );
        assert_eq!(
            tree.get_depth(child_1_1_1),
            Some(2),
            "child_1_1_1 should have depth 2"
        );
        assert_eq!(
            tree.get_depth(child_1_2),
            Some(2),
            "child_1_2 should have depth 1"
        );
        assert_eq!(
            tree.get_depth(child_1_3),
            Some(2),
            "child_1_3 should have depth 2"
        );

        assert_eq!(
            tree.get_depth(child_2),
            Some(1),
            "child_2 should have depth 1"
        );

        assert_eq!(
            tree.get_depth(child_3),
            Some(1),
            "child_3 should have depth 1"
        );
        assert_eq!(
            tree.get_depth(child_3_1),
            Some(2),
            "child_3_1 should have depth 2"
        );
    }
}

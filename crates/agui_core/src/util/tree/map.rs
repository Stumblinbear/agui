use std::collections::VecDeque;

use crate::util::tree::{
    iter::{
        ChildIterator, DownwardIterator, IterableTree, ParentIterator, SubtreeIterator,
        UpwardIterator,
    },
    node::TreeNode,
    storage::{
        sealed::{TreeContainer, TreeContainerAdd, TreeContainerInsert, TreeStorage},
        HopSlotMapStorage,
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
    pub fn remove_subtree(&mut self, node_id: K) -> Option<V> {
        // TODO: optimize this

        // Remove all children
        if let Some(children) = self.get_children(node_id).cloned() {
            for child_id in children.iter() {
                self.remove_subtree(*child_id);
            }
        }

        self.remove(node_id)
    }

    pub fn remove(&mut self, node_id: K) -> Option<V> {
        if self.root == Some(node_id) {
            self.root = None;
        }

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
    /// Moves a node from one parent to another.
    ///
    /// Returns `true` if the node was moved, `false` if the node was already a child of the new parent.
    pub fn reparent(&mut self, new_parent_id: Option<K>, node_id: K) -> bool {
        if new_parent_id.is_none() {
            self.root = Some(node_id);
        } else if self.root == Some(node_id) {
            self.root = None;
        }

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

    // pub fn is_first_child(&self, node_id: K) -> bool {
    //     if let Some(parent_id) = self.get_parent(node_id) {
    //         if let Some(parent) = self.nodes.get(parent_id) {
    //             return parent
    //                 .children
    //                 .first()
    //                 .map_or(true, |child_id| *child_id == node_id);
    //         }
    //     }

    //     true
    // }

    // pub fn is_last_child(&self, node_id: K) -> bool {
    //     if let Some(parent_id) = self.get_parent(node_id) {
    //         if let Some(parent) = self.nodes.get(parent_id) {
    //             return parent
    //                 .children
    //                 .last()
    //                 .map_or(true, |child_id| *child_id == node_id);
    //         }
    //     }

    //     true
    // }
}

impl<K, V, Storage> Tree<K, V, Storage>
where
    K: slotmap::Key,
    Storage: TreeStorage,
{
    // TODO: add our own Iter type instead of using the storage's iter type
    pub fn iter(
        &self,
    ) -> <Storage::Container<K, TreeNode<K, V>> as TreeContainer<K, TreeNode<K, V>>>::Iter<'_> {
        self.nodes.iter()
    }

    pub fn iter_down(&self) -> DownwardIterator<K, Self> {
        DownwardIterator {
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

    pub fn iter_down_from(&self, node_id: K) -> DownwardIterator<K, Self> {
        DownwardIterator {
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

// #[cfg(test)]
// mod tests {
//     use slotmap::HopSlotMap;

//     use crate::{element::ElementId, util::tree::TreeNode};

//     use super::Tree;

//     #[test]
//     fn hierarchy() {
//         let mut tree: Tree<HopSlotMap<ElementId, TreeNode<ElementId, usize>>> =
//             Tree::default();

//         let root_id = tree.add(None, 0);

//         let child_1 = tree.add(Some(root_id), 1);
//         let child_1_1 = tree.add(Some(child_1), 2);
//         let child_1_1_1 = tree.add(Some(child_1_1), 3);
//         let child_1_2 = tree.add(Some(child_1), 4);
//         let child_1_3 = tree.add(Some(child_1), 5);

//         let child_2 = tree.add(Some(root_id), 6);

//         let child_3 = tree.add(Some(root_id), 7);
//         let child_3_1 = tree.add(Some(child_3), 8);

//         assert!(
//             tree.is_first_child(child_1),
//             "child_1 is the first child of the parent"
//         );
//         assert!(
//             !tree.is_last_child(child_1),
//             "child_1 is not the last child of the parent"
//         );

//         assert!(
//             tree.is_first_child(child_1_1),
//             "child_1_1 is the first child of the parent"
//         );
//         assert!(
//             !tree.is_last_child(child_1_1),
//             "child_1_1 is not the last child of the parent"
//         );

//         assert!(
//             tree.is_first_child(child_1_1_1),
//             "child_1_1_1 is the first child of the parent"
//         );
//         assert!(
//             tree.is_last_child(child_1_1_1),
//             "child_1_1_1 is the last child of the parent"
//         );

//         assert!(
//             !tree.is_first_child(child_1_2),
//             "child_1_2 is not the first child of the parent"
//         );
//         assert!(
//             !tree.is_last_child(child_1_2),
//             "child_1_2 is not the last child of the parent"
//         );

//         assert!(
//             !tree.is_first_child(child_1_3),
//             "child_1_3 is not the first child of the parent"
//         );
//         assert!(
//             tree.is_last_child(child_1_3),
//             "child_1_3 is the last child of the parent"
//         );

//         assert!(
//             !tree.is_first_child(child_2),
//             "child_2 is not the first child of the parent"
//         );
//         assert!(
//             !tree.is_last_child(child_2),
//             "child_2 is not the last child of the parent"
//         );

//         assert!(
//             !tree.is_first_child(child_3),
//             "child_3 is not the first child of the parent"
//         );
//         assert!(
//             tree.is_last_child(child_3),
//             "child_3 is the last child of the parent"
//         );

//         assert!(
//             tree.is_first_child(child_3_1),
//             "child_3_1 is the first child of the parent"
//         );
//         assert!(
//             tree.is_last_child(child_3_1),
//             "child_3_1 is the last child of the parent"
//         );
//     }

//     #[test]
//     fn downward_iter() {
//         let mut tree: Tree<HopSlotMap<ElementId, TreeNode<ElementId, usize>>> =
//             Tree::default();

//         let root_id = tree.add(None, 0);

//         let child_1 = tree.add(Some(root_id), 1);
//         let child_1_1 = tree.add(Some(child_1), 2);
//         let child_1_1_1 = tree.add(Some(child_1_1), 3);
//         let child_1_2 = tree.add(Some(child_1), 4);
//         let child_1_3 = tree.add(Some(child_1), 5);

//         let child_2 = tree.add(Some(root_id), 6);

//         let child_3 = tree.add(Some(root_id), 7);
//         let child_3_1 = tree.add(Some(child_3), 8);

//         let mut iter = tree.iter_down_from(root_id);

//         assert_eq!(
//             iter.next(),
//             Some(root_id),
//             "downward iterator's first element must be the root node"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1),
//             "downward iterator should have returned child_1"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1_1),
//             "downward iterator should have returned child_1_1"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1_1_1),
//             "downward iterator should have returned child_1_1_1"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1_2),
//             "downward iterator should have returned child_1_2"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1_3),
//             "downward iterator should have returned child_1_3"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_2),
//             "downward iterator should have returned child_2"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_3),
//             "downward iterator should have returned child_3"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_3_1),
//             "downward iterator should have returned child_3_1"
//         );
//         assert_eq!(
//             iter.next(),
//             None,
//             "downward iterator should have returned None"
//         );
//         assert_eq!(
//             iter.next(),
//             None,
//             "downward iterator should have returned None"
//         );

//         let mut iter = tree.iter_down_from(child_2);

//         assert_eq!(
//             iter.next(),
//             Some(child_2),
//             "downward iterator should have returned child_2"
//         );

//         assert_eq!(
//             iter.next(),
//             Some(child_3),
//             "downward iterator should have returned child_3"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_3_1),
//             "downward iterator should have returned child_3_1"
//         );
//         assert_eq!(
//             iter.next(),
//             None,
//             "downward iterator should have returned None"
//         );
//         assert_eq!(
//             iter.next(),
//             None,
//             "downward iterator should have returned None"
//         );

//         let mut iter = tree.iter_down_from(child_3);

//         assert_eq!(
//             iter.next(),
//             Some(child_3),
//             "downward iterator should have returned child_3"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_3_1),
//             "downward iterator should have returned child_3_1"
//         );
//         assert_eq!(
//             iter.next(),
//             None,
//             "downward iterator should have returned None"
//         );
//     }

//     #[test]
//     fn upward_iter() {
//         let mut tree: Tree<HopSlotMap<ElementId, TreeNode<ElementId, usize>>> =
//             Tree::default();

//         let root_id = tree.add(None, 0);

//         let child_1 = tree.add(Some(root_id), 1);
//         let child_1_1 = tree.add(Some(child_1), 2);
//         let child_1_1_1 = tree.add(Some(child_1_1), 3);
//         let child_1_2 = tree.add(Some(child_1), 4);
//         let child_1_3 = tree.add(Some(child_1), 5);

//         let child_2 = tree.add(Some(root_id), 6);

//         let child_3 = tree.add(Some(root_id), 7);
//         let child_3_1 = tree.add(Some(child_3), 8);

//         let mut iter = tree.iter_up_from(child_3_1);

//         assert_eq!(
//             iter.next(),
//             Some(child_3_1),
//             "upward iterator should have returned child_3_1"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_3),
//             "upward iterator should have returned child_3"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_2),
//             "upward iterator should have returned child_2"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1_3),
//             "upward iterator should have returned child_1_3"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1_2),
//             "upward iterator should have returned child_1_2"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1_1_1),
//             "upward iterator should have returned child_1_1_1"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1_1),
//             "upward iterator should have returned child_1_1"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1),
//             "upward iterator should have returned child_1"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(root_id),
//             "upward iterator should have returned the root node"
//         );
//         assert_eq!(
//             iter.next(),
//             None,
//             "upward iterator should have returned None"
//         );
//         assert_eq!(
//             iter.next(),
//             None,
//             "upward iterator should have returned None"
//         );

//         let mut iter = tree.iter_up_from(child_1_2);

//         assert_eq!(
//             iter.next(),
//             Some(child_1_2),
//             "upward iterator should have returned child_1_2"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1_1_1),
//             "upward iterator should have returned child_1_1_1"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1_1),
//             "upward iterator should have returned child_1_1"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1),
//             "upward iterator should have returned child_1"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(root_id),
//             "upward iterator should have returned the root node"
//         );
//         assert_eq!(
//             iter.next(),
//             None,
//             "upward iterator should have returned None"
//         );
//     }

//     #[test]
//     fn subtree_iter() {
//         let mut tree: Tree<HopSlotMap<ElementId, TreeNode<ElementId, usize>>> =
//             Tree::default();

//         let root_id = tree.add(None, 0);

//         let child_1 = tree.add(Some(root_id), 1);
//         let child_1_1 = tree.add(Some(child_1), 2);
//         let child_1_1_1 = tree.add(Some(child_1_1), 3);
//         let child_1_2 = tree.add(Some(child_1), 4);
//         let child_1_3 = tree.add(Some(child_1), 5);

//         let child_2 = tree.add(Some(root_id), 6);

//         let child_3 = tree.add(Some(root_id), 7);
//         let child_3_1 = tree.add(Some(child_3), 8);

//         let mut iter = tree.iter_subtree(child_1, |_| true);

//         assert_eq!(
//             iter.next(),
//             Some(child_1),
//             "subtree iterator's first element must be child_1"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1_1),
//             "subtree iterator should have returned child_1_1"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1_1_1),
//             "subtree iterator should have returned child_1_1_1"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1_2),
//             "subtree iterator should have returned child_1_2"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1_3),
//             "subtree iterator should have returned child_1_3"
//         );
//         assert_eq!(
//             iter.next(),
//             None,
//             "subtree iterator should have returned None"
//         );

//         let mut iter = tree.iter_subtree(child_2, |_| true);

//         assert_eq!(
//             iter.next(),
//             Some(child_2),
//             "subtree iterator should have returned child_2"
//         );
//         assert_eq!(
//             iter.next(),
//             None,
//             "subtree iterator should have returned None"
//         );

//         let mut iter = tree.iter_subtree(child_3, |_| true);

//         assert_eq!(
//             iter.next(),
//             Some(child_3),
//             "subtree iterator should have returned child_3"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_3_1),
//             "subtree iterator should have returned child_3_1"
//         );
//         assert_eq!(
//             iter.next(),
//             None,
//             "subtree iterator should have returned None"
//         );
//     }

//     #[test]
//     fn depth_propagation() {
//         let mut tree: Tree<HopSlotMap<ElementId, TreeNode<ElementId, usize>>> =
//             Tree::default();

//         let root_id = tree.add(None, 0);

//         let child_1 = tree.add(Some(root_id), 1);
//         let child_1_1 = tree.add(Some(child_1), 2);
//         let child_1_1_1 = tree.add(Some(child_1_1), 3);
//         let child_1_2 = tree.add(Some(child_1), 4);
//         let child_1_3 = tree.add(Some(child_1), 5);

//         let child_2 = tree.add(Some(root_id), 6);

//         let child_3 = tree.add(Some(root_id), 7);
//         let child_3_1 = tree.add(Some(child_3), 8);

//         assert_eq!(
//             tree.get_depth(root_id),
//             Some(0),
//             "root node should have depth 0"
//         );

//         assert_eq!(
//             tree.get_depth(child_1),
//             Some(1),
//             "child_1 should have depth 1"
//         );
//         assert_eq!(
//             tree.get_depth(child_1_1),
//             Some(2),
//             "child_1_1 should have depth 2"
//         );
//         assert_eq!(
//             tree.get_depth(child_1_1_1),
//             Some(3),
//             "child_1_1_1 should have depth 3"
//         );
//         assert_eq!(
//             tree.get_depth(child_1_2),
//             Some(2),
//             "child_1_2 should have depth 2"
//         );
//         assert_eq!(
//             tree.get_depth(child_1_3),
//             Some(2),
//             "child_1_3 should have depth 2"
//         );

//         assert_eq!(
//             tree.get_depth(child_2),
//             Some(1),
//             "child_2 should have depth 1"
//         );

//         assert_eq!(
//             tree.get_depth(child_3),
//             Some(1),
//             "child_3 should have depth 1"
//         );
//         assert_eq!(
//             tree.get_depth(child_3_1),
//             Some(2),
//             "child_3_1 should have depth 2"
//         );

//         tree.reparent(Some(root_id), child_1_1);

//         assert_eq!(
//             tree.get_depth(root_id),
//             Some(0),
//             "root node should have depth 0"
//         );

//         assert_eq!(
//             tree.get_depth(child_1),
//             Some(1),
//             "child_1 should have depth 1"
//         );
//         assert_eq!(
//             tree.get_depth(child_1_1),
//             Some(1),
//             "child_1_1 should have depth 1"
//         );
//         assert_eq!(
//             tree.get_depth(child_1_1_1),
//             Some(2),
//             "child_1_1_1 should have depth 2"
//         );
//         assert_eq!(
//             tree.get_depth(child_1_2),
//             Some(2),
//             "child_1_2 should have depth 1"
//         );
//         assert_eq!(
//             tree.get_depth(child_1_3),
//             Some(2),
//             "child_1_3 should have depth 2"
//         );

//         assert_eq!(
//             tree.get_depth(child_2),
//             Some(1),
//             "child_2 should have depth 1"
//         );

//         assert_eq!(
//             tree.get_depth(child_3),
//             Some(1),
//             "child_3 should have depth 1"
//         );
//         assert_eq!(
//             tree.get_depth(child_3_1),
//             Some(2),
//             "child_3_1 should have depth 2"
//         );
//     }
// }

// #[cfg(test)]
// mod tests {
//     use crate::element::ElementId;

//     use super::Tree;

//     #[test]
//     fn is_first_last_child() {
//         let mut tree: Tree<ElementId, usize> = Tree::default();

//         let root_id = tree.add(None, 0);

//         assert!(
//             tree.map.is_first_child(root_id),
//             "root should return true for first child checks"
//         );
//         assert!(
//             tree.map.is_last_child(root_id),
//             "root should return true for last child checks"
//         );
//     }

//     #[test]
//     fn downward_iter() {
//         let mut tree: Tree<ElementId, usize> = Tree::default();

//         let root_id = tree.add(None, 0);

//         let child_1 = tree.add(Some(root_id), 1);
//         let child_1_1 = tree.add(Some(child_1), 2);
//         tree.add(Some(child_1_1), 3);
//         tree.add(Some(child_1), 4);
//         tree.add(Some(child_1), 5);

//         tree.add(Some(root_id), 6);

//         let child_3 = tree.add(Some(root_id), 7);
//         tree.add(Some(child_3), 8);

//         let mut iter = tree.iter_down();

//         assert_eq!(
//             iter.next(),
//             Some(root_id),
//             "downward iterator's first element must be the root node"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_1),
//             "downward iterator should have returned child_1"
//         );
//     }

//     #[test]
//     fn upward_iter() {
//         let mut tree: Tree<ElementId, usize> = Tree::default();

//         let root_id = tree.add(None, 0);

//         let child_1 = tree.add(Some(root_id), 1);
//         let child_1_1 = tree.add(Some(child_1), 2);
//         tree.add(Some(child_1_1), 3);
//         tree.add(Some(child_1), 4);
//         tree.add(Some(child_1), 5);

//         tree.add(Some(root_id), 6);

//         let child_3 = tree.add(Some(root_id), 7);
//         let child_3_1 = tree.add(Some(child_3), 8);

//         let mut iter = tree.iter_up();

//         assert_eq!(
//             iter.next(),
//             Some(child_3_1),
//             "upward iterator should have returned child_3_1"
//         );
//         assert_eq!(
//             iter.next(),
//             Some(child_3),
//             "upward iterator should have returned child_3"
//         );
//     }
// }

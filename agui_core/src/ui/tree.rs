use std::{collections::HashMap, hash::Hash};

use morphorm::Hierarchy;

#[derive(Debug)]
pub struct Tree<K> {
    root: Option<K>,
    nodes: HashMap<K, TreeNode<K>>,
}

impl<K> Default for Tree<K> {
    fn default() -> Self {
        Self {
            root: None,
            nodes: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct TreeNode<K> {
    pub(crate) depth: usize,

    pub(crate) parent: Option<K>,
    pub(crate) children: Vec<K>,
}

impl<K> Tree<K>
where
    K: Copy + PartialEq + Eq + Hash,
{
    #[allow(dead_code)]
    pub fn get_root(&self) -> Option<K> {
        self.root
    }

    pub fn add(&mut self, parent_id: Option<K>, node_id: K) -> K {
        let mut depth = 0;

        if let Some(parent_id) = parent_id {
            if let Some(parent) = self.nodes.get_mut(&parent_id) {
                depth = parent.depth + 1;

                parent.children.push(node_id);
            } else {
                panic!("cannot add a node to a parent that doesn't exist");
            }
        } else {
            if parent_id.is_some() {
                panic!("root node cannot have a parent")
            }

            self.root = Some(node_id);
        }

        self.nodes.insert(
            node_id,
            TreeNode {
                depth,
                parent: parent_id,
                children: Vec::new(),
            },
        );

        node_id
    }

    pub fn remove(&mut self, node_id: &K) -> Option<TreeNode<K>> {
        if let Some(node) = self.nodes.remove(node_id) {
            for child in &node.children {
                self.nodes.remove(child);
            }

            Some(node)
        } else {
            None
        }
    }

    pub fn get(&self, node_id: &K) -> Option<&TreeNode<K>> {
        self.nodes.get(node_id)
    }

    pub fn iter(&self) -> DownwardIterator<K> {
        DownwardIterator {
            tree: self,
            node_id: self.root,
            first: true,
        }
    }

    pub fn iter_from(&self, node_id: K) -> DownwardIterator<K> {
        DownwardIterator {
            tree: self,
            node_id: Some(node_id),
            first: true,
        }
    }

    pub fn iter_up(&self) -> UpwardIterator<K> {
        UpwardIterator {
            tree: self,
            node_id: self.get_deepest_child(self.root),
            first: true,
        }
    }

    #[allow(dead_code)]
    pub fn iter_up_from(&self, node_id: K) -> UpwardIterator<K> {
        UpwardIterator {
            tree: self,
            node_id: Some(node_id),
            first: true,
        }
    }

    pub fn has_child(&self, node_id: &K, child_id: &K) -> bool {
        let node = self.get(node_id);
        let child = self.get(child_id);

        // Make sure they're actually in the tree
        if node.is_none() || child.is_none() {
            return false;
        }

        let child_depth = child.unwrap().depth;

        // If the node's depth is below the child, it's impossible for the child to be in the parent
        if node.unwrap().depth >= child_depth {
            return false;
        }

        let child_id = *child_id;

        for node_id in self.iter_from(*node_id) {
            let node = self.get(&node_id).expect("tree broken");

            // If we reach a depth lower than the child, bail, because the child won't be found. We do
            // not do an equality check, here, because we may find the child as a sibling
            if node.depth > child_depth {
                return false;
            }

            // The child exists under the parent
            if node_id == child_id {
                return true;
            }
        }

        false
    }

    fn get_deepest_child(&self, mut current_node_id: Option<K>) -> Option<K> {
        while let Some(node_id) = current_node_id {
            if let Some(node) = self.nodes.get(&node_id) {
                if let Some(last_child) = node.children.last() {
                    current_node_id = Some(*last_child);
                } else {
                    // If it has no children, this is the last node in the tree
                    break;
                }
            }
        }

        current_node_id
    }

    #[allow(clippy::unused_self)]
    fn get_next_sibling(&self, parent: &TreeNode<K>, sibling_id: K) -> Option<K> {
        let mut children = parent.children.iter();

        while let Some(child_id) = children.next() {
            if *child_id == sibling_id {
                let child_id = children.next();

                if let Some(child_id) = child_id {
                    return Some(*child_id);
                }

                return None;
            }
        }

        None
    }

    #[allow(clippy::unused_self)]
    fn get_prev_sibling(&self, parent: &TreeNode<K>, sibling_id: K) -> Option<K> {
        let mut last_child_id = None;

        for child_id in &parent.children {
            if *child_id == sibling_id {
                return last_child_id;
            }

            last_child_id = Some(*child_id);
        }

        last_child_id
    }
}

pub struct DownwardIterator<'a, K> {
    tree: &'a Tree<K>,
    node_id: Option<K>,
    first: bool,
}

impl<'a, K> Iterator for DownwardIterator<'a, K>
where
    K: Copy + PartialEq + Eq + Hash,
{
    type Item = K;

    fn next(&mut self) -> Option<K> {
        if self.first {
            self.first = false;
            return self.node_id;
        }

        if let Some(node_id) = self.node_id {
            // Grab the node from the tree
            if let Some(node) = self.tree.get(&node_id) {
                // Grab the first child node
                if let Some(child_id) = node.children.first() {
                    self.node_id = Some(*child_id);
                } else {
                    let mut current_parent = node.parent;
                    let mut after_child_id = node_id;

                    loop {
                        // If we have no children, return the sibling after the node_id
                        if let Some(parent_node_id) = current_parent {
                            if let Some(parent_node) = self.tree.get(&parent_node_id) {
                                if let Some(sibling_id) =
                                    self.tree.get_next_sibling(parent_node, after_child_id)
                                {
                                    self.node_id = Some(sibling_id);
                                    break;
                                }

                                // Move up to to the parent to check its next child
                                current_parent = parent_node.parent;

                                // Set after_child_id to parent_node_id so it's skipped
                                after_child_id = parent_node_id;
                            } else {
                                // Parent doesn't exist in the tree. Bail.
                                self.node_id = None;
                                break;
                            }
                        } else {
                            // Has no parent. Bail.
                            self.node_id = None;
                            break;
                        }
                    }
                }
            } else {
                // If the node doesn't exist in the tree, then there's nothing to iterate
                self.node_id = None;
            }
        }

        self.node_id
    }
}

pub struct UpwardIterator<'a, K> {
    tree: &'a Tree<K>,
    node_id: Option<K>,
    first: bool,
}

impl<'a, K> Iterator for UpwardIterator<'a, K>
where
    K: Copy + PartialEq + Eq + Hash,
{
    type Item = K;

    fn next(&mut self) -> Option<K> {
        if self.first {
            self.first = false;
            return self.node_id;
        }

        if let Some(node_id) = self.node_id {
            // Grab the node from the tree
            if let Some(node) = self.tree.get(&node_id) {
                if let Some(parent_node_id) = node.parent {
                    if let Some(parent_node) = self.tree.get(&parent_node_id) {
                        let first_child_id = parent_node.children.first().unwrap();

                        // If we're the first child, then return the parent
                        if node_id == *first_child_id {
                            self.node_id = node.parent;
                        } else {
                            // Grab the previous sibling's deepest child
                            let sibling_id = self.tree.get_prev_sibling(parent_node, node_id);

                            self.node_id = self.tree.get_deepest_child(sibling_id);
                        }
                    }
                } else {
                    // TreeNode doesn't have a parent, so we're at the root.
                    self.node_id = None;
                }
            } else {
                // If the node doesn't exist in the tree, then there's nothing to iterate
                self.node_id = None;
            }
        }

        self.node_id
    }
}

pub struct ChildIterator<'a, K> {
    tree: &'a Tree<K>,
    node_id: K,
    current_child_id: Option<K>,
    first: bool,
}

impl<'a, K: 'a> Iterator for ChildIterator<'a, K>
where
    K: Copy + PartialEq + Eq + Hash,
{
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.tree.get(&self.node_id) {
            let mut children = node.children.iter();

            if let Some(current_child_id) = self.current_child_id {
                self.current_child_id = None;

                while let Some(child_id) = children.next() {
                    if *child_id == current_child_id {
                        let child_id = children.next();

                        if let Some(child_id) = child_id {
                            self.current_child_id = Some(*child_id);
                        } else {
                            self.current_child_id = None;
                        }

                        break;
                    }
                }
            } else if self.first {
                self.first = false;

                let child_id = children.next();

                if let Some(child_id) = child_id {
                    self.current_child_id = Some(*child_id);
                } else {
                    self.current_child_id = None;
                }
            }

            return self.current_child_id;
        }

        None
    }
}

impl<'a, K: 'a> Hierarchy<'a> for Tree<K>
where
    K: PartialEq + Eq + Hash + for<'b> morphorm::Node<'b>,
{
    type Item = K;
    type DownIter = DownwardIterator<'a, K>;
    type UpIter = UpwardIterator<'a, K>;
    type ChildIter = ChildIterator<'a, K>;

    fn up_iter(&'a self) -> Self::UpIter {
        self.iter_up()
    }

    fn down_iter(&'a self) -> Self::DownIter {
        self.iter()
    }

    fn child_iter(&'a self, node_id: Self::Item) -> Self::ChildIter {
        ChildIterator {
            tree: self,
            node_id,
            current_child_id: None,
            first: true,
        }
    }

    fn parent(&self, node_id: Self::Item) -> Option<Self::Item> {
        if let Some(parent) = self.get(&node_id) {
            return parent.parent;
        }

        None
    }

    fn is_first_child(&self, node_id: Self::Item) -> bool {
        if let Some(parent_id) = self.parent(node_id) {
            if let Some(parent) = self.get(&parent_id) {
                return parent
                    .children
                    .first()
                    .map_or(false, |child_id| *child_id == node_id);
            }
        }

        false
    }

    fn is_last_child(&self, node_id: Self::Item) -> bool {
        if let Some(parent_id) = self.parent(node_id) {
            if let Some(parent) = self.get(&parent_id) {
                return parent
                    .children
                    .last()
                    .map_or(false, |child_id| *child_id == node_id);
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use crate::widget::WidgetID;

    use super::Tree;

    #[test]
    fn test_downward_iter() {
        let mut tree: Tree<WidgetID> = Tree::default();

        let root_id = tree.add(None, 0.into());

        let child_1 = tree.add(Some(root_id), 1.into());
        let child_1_1 = tree.add(Some(root_id), 2.into());
        let child_1_2 = tree.add(Some(root_id), 3.into());
        let child_1_3 = tree.add(Some(root_id), 4.into());

        let child_2 = tree.add(Some(root_id), 5.into());

        let child_3 = tree.add(Some(root_id), 6.into());
        let child_3_1 = tree.add(Some(root_id), 7.into());

        let mut iter = tree.iter();

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

        let mut iter = tree.iter_from(child_3);

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
    fn test_upward_iter() {
        let mut tree: Tree<WidgetID> = Tree::default();

        let root_id = tree.add(None, 0.into());

        let child_1 = tree.add(Some(root_id), 1.into());
        let child_1_1 = tree.add(Some(root_id), 2.into());
        let child_1_2 = tree.add(Some(root_id), 3.into());
        let child_1_3 = tree.add(Some(root_id), 4.into());

        let child_2 = tree.add(Some(root_id), 5.into());

        let child_3 = tree.add(Some(root_id), 6.into());
        let child_3_1 = tree.add(Some(root_id), 7.into());

        let mut iter = tree.iter_up();

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
}

use crate::util::tree::TreeNode;

pub trait IterableTree<K>
where
    K: slotmap::Key,
{
    type Value;

    fn get_node(&self, node_id: K) -> Option<&TreeNode<K, Self::Value>>;

    fn get_next_sibling(&self, parent_id: K, sibling_id: K) -> Option<K> {
        if let Some(parent) = self.get_node(parent_id) {
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
        }

        None
    }

    fn get_prev_sibling(&self, parent_id: K, sibling_id: K) -> Option<K> {
        if let Some(parent) = self.get_node(parent_id) {
            let mut last_child_id = None;

            for child_id in &parent.children {
                if *child_id == sibling_id {
                    return last_child_id;
                }

                last_child_id = Some(*child_id);
            }
        }

        None
    }
}

pub struct DownwardIterator<'a, K, I> {
    pub(super) tree: &'a I,
    pub(super) node_id: Option<K>,
    pub(super) first: bool,
}

impl<'a, K, I> Iterator for DownwardIterator<'a, K, I>
where
    K: slotmap::Key,
    I: IterableTree<K>,
{
    type Item = K;

    fn next(&mut self) -> Option<K> {
        if self.first {
            self.first = false;
            return self.node_id;
        }

        if let Some(node_id) = self.node_id {
            // Grab the node from the tree
            if let Some(node) = self.tree.get_node(node_id) {
                // Grab the first child node
                if let Some(child_id) = node.children.first() {
                    self.node_id = Some(*child_id);

                    return self.node_id;
                }

                let mut current_parent = node.parent;
                let mut after_child_id = node_id;

                loop {
                    // If we have no children, return the sibling after the node_id
                    if let Some(parent_node_id) = current_parent {
                        if let Some(sibling_id) =
                            self.tree.get_next_sibling(parent_node_id, after_child_id)
                        {
                            self.node_id = Some(sibling_id);
                            break;
                        } else {
                            // Move up to to the parent to check its next child
                            current_parent = self
                                .tree
                                .get_node(parent_node_id)
                                .and_then(|node| node.parent);

                            // Set after_child_id to parent_node_id so it's skipped
                            after_child_id = parent_node_id;
                        }
                    } else {
                        // Has no parent. Bail.
                        self.node_id = None;
                        break;
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

pub struct SubtreeIterator<'a, K, F, I> {
    pub(super) tree: &'a I,
    pub(super) root_node_id: K,
    pub(super) node_id: Option<K>,
    pub(super) first: bool,
    pub(super) filter: F,
}

impl<'a, K, F, I> Iterator for SubtreeIterator<'a, K, F, I>
where
    K: slotmap::Key,
    F: Fn(K) -> bool,
    I: IterableTree<K>,
{
    type Item = K;

    fn next(&mut self) -> Option<K> {
        if self.first {
            self.first = false;

            if let Some(node_id) = self.node_id {
                if (self.filter)(node_id) {
                    return self.node_id;
                } else {
                    self.node_id = None;
                }
            }
        }

        if let Some(node_id) = self.node_id {
            // Grab the node from the tree
            if let Some(node) = self.tree.get_node(node_id) {
                // Grab the first child node
                if let Some(child_id) = node.children.first() {
                    // If the child passes the filter, return it
                    if (self.filter)(*child_id) {
                        self.node_id = Some(*child_id);

                        return self.node_id;
                    }
                }

                let mut current_parent = node.parent;
                let mut after_child_id = node_id;

                'find_child: loop {
                    // We're iterating a subtree, so we don't want to go above the defined root node
                    if after_child_id == self.root_node_id {
                        self.node_id = None;
                        break 'find_child;
                    }

                    // If we have no children, return the sibling after the node_id
                    if let Some(parent_node_id) = current_parent {
                        // Check each sibling of the parent for ones that pass the filter
                        while let Some(sibling_id) =
                            self.tree.get_next_sibling(parent_node_id, after_child_id)
                        {
                            if (self.filter)(sibling_id) {
                                self.node_id = Some(sibling_id);
                                break 'find_child;
                            }

                            after_child_id = sibling_id;
                        }

                        // Move up to to the parent to check its next child
                        current_parent = self
                            .tree
                            .get_node(parent_node_id)
                            .and_then(|node| node.parent);

                        // Set after_child_id to parent_node_id so it's skipped
                        after_child_id = parent_node_id;
                    } else {
                        // Has no parent. Bail.
                        self.node_id = None;
                        break 'find_child;
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

pub struct UpwardIterator<'a, K, I> {
    pub(super) tree: &'a I,
    pub(super) node_id: Option<K>,
    pub(super) first: bool,
}

impl<'a, K, I> UpwardIterator<'a, K, I>
where
    K: slotmap::Key,
    I: IterableTree<K>,
{
    pub fn start_at(tree: &'a I, node_id: K) -> Self {
        Self {
            tree,
            node_id: Some(node_id),
            first: true,
        }
    }

    pub fn from_root(tree: &'a I, node_id: Option<K>) -> Self {
        let mut iter = Self {
            tree,
            node_id: None,
            first: true,
        };

        iter.node_id = iter.get_furthest_child(node_id);

        iter
    }

    fn get_furthest_child(&self, mut current_node_id: Option<K>) -> Option<K> {
        while let Some(node_id) = current_node_id {
            if let Some(node) = self.tree.get_node(node_id) {
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
}

impl<'a, K, I> Iterator for UpwardIterator<'a, K, I>
where
    K: slotmap::Key,
    I: IterableTree<K>,
{
    type Item = K;

    fn next(&mut self) -> Option<K> {
        if self.first {
            self.first = false;
            return self.node_id;
        }

        if let Some(node_id) = self.node_id {
            // Grab the node from the tree
            if let Some(node) = self.tree.get_node(node_id) {
                if let Some(parent_node_id) = node.parent {
                    if let Some(sibling_id) = self.tree.get_prev_sibling(parent_node_id, node_id) {
                        self.node_id = self.get_furthest_child(Some(sibling_id));
                    } else {
                        self.node_id = node.parent;
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

pub struct ParentIterator<'a, K, I> {
    pub(super) tree: &'a I,
    pub(super) node_id: Option<K>,
}

impl<'a, K, I> Iterator for ParentIterator<'a, K, I>
where
    K: slotmap::Key,
    I: IterableTree<K>,
{
    type Item = K;

    fn next(&mut self) -> Option<K> {
        if let Some(node_id) = self.node_id {
            // Grab the node from the tree
            if let Some(node) = self.tree.get_node(node_id) {
                self.node_id = node.parent;
            } else {
                // If the node doesn't exist in the tree, then there's nothing to iterate
                self.node_id = None;
            }
        }

        self.node_id
    }
}

pub struct ChildIterator<'a, K, I> {
    pub(super) tree: &'a I,
    pub(super) node_id: K,
    pub(super) current_child_id: Option<K>,
    pub(super) first: bool,
}

impl<'a, K, I> Iterator for ChildIterator<'a, K, I>
where
    K: slotmap::Key,
    I: IterableTree<K>,
{
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.tree.get_node(self.node_id) {
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

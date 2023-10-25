use std::ops::{Index, IndexMut};

use slotmap::Key;

use self::map::{ChildIterator, DownwardIterator, TreeMap, UpwardIterator};

mod forest;
mod map;

pub use forest::Forest;
pub use map::TreeNode;
pub use slotmap::new_key_type;

pub struct Tree<K, V>
where
    K: Key,
{
    root: Option<K>,
    map: TreeMap<K, V>,
}

impl<K, V> Default for Tree<K, V>
where
    K: Key,
{
    fn default() -> Self {
        Self {
            root: None,
            map: TreeMap::default(),
        }
    }
}

impl<K, V> std::ops::Deref for Tree<K, V>
where
    K: Key,
{
    type Target = TreeMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<K, V> std::ops::DerefMut for Tree<K, V>
where
    K: Key,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl<K, V> Tree<K, V>
where
    K: Key,
{
    pub fn root(&self) -> Option<K> {
        self.root
    }

    pub fn clear(&mut self) {
        self.root = None;
        self.map.clear();
    }

    pub fn add(&mut self, parent_id: Option<K>, value: V) -> K {
        let node_id = self.map.add(parent_id, value);

        if parent_id.is_none() {
            self.root = Some(node_id);
        }

        node_id
    }

    pub fn remove(&mut self, node_id: K, cascade: bool) -> Option<V> {
        if self.root == Some(node_id) {
            self.root = None;
        }

        if cascade {
            // Remove all children
            if let Some(children) = self.map.get_children(node_id).cloned() {
                for child_id in children.iter() {
                    self.remove(*child_id, cascade);
                }
            }
        }

        self.map.remove(node_id)
    }

    pub fn with<F, R>(&mut self, node_id: K, func: F) -> Option<R>
    where
        F: FnOnce(&mut Tree<K, V>, &mut V) -> R,
    {
        if let Some(mut value) = self.take(node_id) {
            let ret = func(self, &mut value);

            self.replace(node_id, value);

            Some(ret)
        } else {
            None
        }
    }

    pub fn reparent(&mut self, new_parent_id: Option<K>, node_id: K) -> bool {
        if self.root == Some(node_id) {
            self.root = None;
        }

        self.map.reparent(new_parent_id, node_id)
    }

    pub fn iter_down(&self) -> DownwardIterator<K, V> {
        DownwardIterator {
            tree: &self.map,
            node_id: self.root,
            first: true,
        }
    }

    pub fn iter_up(&self) -> UpwardIterator<K, V> {
        UpwardIterator {
            tree: self,
            node_id: self.get_deepest_child(self.root),
            first: true,
        }
    }

    pub fn iter_children(&self, node_id: K) -> ChildIterator<K, V> {
        ChildIterator {
            tree: self,
            node_id,
            current_child_id: None,
            first: true,
        }
    }
}

impl<K, V> Index<K> for Tree<K, V>
where
    K: Key,
{
    type Output = V;

    fn index(&self, key: K) -> &Self::Output {
        &self.map[key]
    }
}

impl<K, V> IndexMut<K> for Tree<K, V>
where
    K: Key,
{
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        &mut self.map[key]
    }
}

impl<K, V> std::fmt::Debug for Tree<K, V>
where
    K: Key,
    V: std::fmt::Debug,
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

            self[node_id].fmt(f)?;

            writeln!(f, " (#{:?})", node_id)?;
        }

        Ok(())
    }
}

impl<K, V> std::fmt::Display for Tree<K, V>
where
    K: Key,
    V: std::fmt::Display,
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

            self[node_id].fmt(f)?;

            writeln!(f, " (#{:?})", node_id)?;
        }

        Ok(())
    }
}

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
    fn downward_iter() {
        let mut tree: Tree<ElementId, usize> = Tree::default();

        let root_id = tree.add(None, 0);

        let child_1 = tree.add(Some(root_id), 1);
        let child_1_1 = tree.add(Some(child_1), 2);
        tree.add(Some(child_1_1), 3);
        tree.add(Some(child_1), 4);
        tree.add(Some(child_1), 5);

        tree.add(Some(root_id), 6);

        let child_3 = tree.add(Some(root_id), 7);
        tree.add(Some(child_3), 8);

        let mut iter = tree.iter_down();

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
    }

    #[test]
    fn upward_iter() {
        let mut tree: Tree<ElementId, usize> = Tree::default();

        let root_id = tree.add(None, 0);

        let child_1 = tree.add(Some(root_id), 1);
        let child_1_1 = tree.add(Some(child_1), 2);
        tree.add(Some(child_1_1), 3);
        tree.add(Some(child_1), 4);
        tree.add(Some(child_1), 5);

        tree.add(Some(root_id), 6);

        let child_3 = tree.add(Some(root_id), 7);
        let child_3_1 = tree.add(Some(child_3), 8);

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
    }
}

use std::ops::{Index, IndexMut};

use fnv::FnvHashSet;

use slotmap::Key;

use super::map::TreeMap;

pub struct Forest<K, V>
where
    K: Key,
{
    roots: FnvHashSet<K>,
    map: TreeMap<K, V>,
}

impl<K, V> Default for Forest<K, V>
where
    K: Key,
{
    fn default() -> Self {
        Self {
            roots: FnvHashSet::default(),
            map: TreeMap::default(),
        }
    }
}

impl<K, V> std::ops::Deref for Forest<K, V>
where
    K: Key,
{
    type Target = TreeMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<K, V> std::ops::DerefMut for Forest<K, V>
where
    K: Key,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl<K, V> Forest<K, V>
where
    K: Key,
{
    pub fn get_roots(&self) -> &FnvHashSet<K> {
        &self.roots
    }

    pub fn clear(&mut self) {
        self.roots.clear();
        self.map.clear();
    }

    pub fn add(&mut self, parent_id: Option<K>, value: V) -> K {
        let node_id = self.map.add(parent_id, value);

        if parent_id.is_none() {
            // If it has no parent, it is a root
            self.roots.insert(node_id);
        }

        node_id
    }

    pub fn remove(&mut self, node_id: K, cascade: bool) -> Option<V> {
        if self.roots.contains(&node_id) {
            self.roots.remove(&node_id);

            if !cascade {
                // If we're not cascading, all children will be promoted to roots
                if let Some(children) = self.map.get_children(node_id) {
                    self.roots.extend(children);
                }
            }
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

    pub fn reparent(&mut self, new_parent_id: Option<K>, node_id: K) {
        if new_parent_id.is_none() {
            self.roots.insert(node_id);
        } else {
            self.roots.remove(&node_id);
        }

        self.map.reparent(new_parent_id, node_id);
    }
}

impl<K, V> Index<K> for Forest<K, V>
where
    K: Key,
{
    type Output = V;

    fn index(&self, key: K) -> &Self::Output {
        &self.map[key]
    }
}

impl<K, V> IndexMut<K> for Forest<K, V>
where
    K: Key,
{
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        &mut self.map[key]
    }
}

impl<K, V> std::fmt::Debug for Forest<K, V>
where
    K: Key,
    V: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Forest")?;

        for root_id in &self.roots {
            for node_id in self.iter_down_from(*root_id) {
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
        }

        Ok(())
    }
}

impl<K, V> std::fmt::Display for Forest<K, V>
where
    K: Key,
    V: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Forest")?;

        for root_id in &self.roots {
            for node_id in self.iter_down_from(*root_id) {
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
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::widget::WidgetId;

    use super::Forest;

    #[test]
    fn is_first_last_child() {
        let mut tree: Forest<WidgetId, usize> = Forest::default();

        let root_1_id = tree.add(None, 0);

        let child_1 = tree.add(Some(root_1_id), 1);
        let child_1_1 = tree.add(Some(child_1), 2);
        tree.add(Some(child_1_1), 3);
        tree.add(Some(child_1), 4);
        tree.add(Some(child_1), 5);

        let child_2 = tree.add(Some(root_1_id), 6);

        let root_2_id = tree.add(None, 7);

        let child_3 = tree.add(Some(root_2_id), 8);
        tree.add(Some(child_3), 9);

        assert!(
            !tree.is_first_child(root_1_id),
            "roots should return false for first child checks"
        );
        assert!(
            !tree.is_last_child(root_1_id),
            "roots should return false for last child checks"
        );

        assert!(
            !tree.is_first_child(root_2_id),
            "roots should return false for first child checks"
        );
        assert!(
            !tree.is_last_child(root_2_id),
            "roots should return false for last child checks"
        );

        assert!(
            tree.is_first_child(child_1),
            "child_1 is the first child of root_1"
        );
        assert!(
            !tree.is_last_child(child_1),
            "child_1 is not the last child of root_1"
        );
        assert!(
            tree.is_last_child(child_2),
            "child_2 is the last child of root_1"
        );

        assert!(
            tree.is_first_child(child_3),
            "child_3 is the first child of root_2"
        );
        assert!(
            tree.is_last_child(child_3),
            "child_3 is the last child of root_2"
        );
    }
}

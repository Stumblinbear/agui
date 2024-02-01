use crate::util::tree::errors::NodeInUse;

#[derive(Debug)]
pub struct TreeNode<K, V> {
    pub(super) depth: usize,
    pub(super) value: Option<V>,

    pub(super) parent: Option<K>,
    pub(super) children: Vec<K>,
}

impl<K, V> TreeNode<K, V> {
    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn value(&self) -> Result<&V, NodeInUse> {
        self.value.as_ref().ok_or(NodeInUse)
    }

    pub fn value_mut(&mut self) -> Result<&mut V, NodeInUse> {
        self.value.as_mut().ok_or(NodeInUse)
    }

    pub fn take(mut self) -> Result<V, NodeInUse> {
        self.value.take().ok_or(NodeInUse)
    }
}

impl<K, V> TreeNode<K, V>
where
    K: slotmap::Key,
{
    pub fn parent(&self) -> Option<K> {
        self.parent
    }

    pub fn children(&self) -> &[K] {
        &self.children
    }
}

impl<K, V> AsRef<V> for TreeNode<K, V>
where
    K: slotmap::Key,
{
    fn as_ref(&self) -> &V {
        self.value.as_ref().expect("node is currently in use")
    }
}

impl<K, V> AsMut<V> for TreeNode<K, V>
where
    K: slotmap::Key,
{
    fn as_mut(&mut self) -> &mut V {
        self.value.as_mut().expect("node is currently in use")
    }
}

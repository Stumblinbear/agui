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

    #[inline]
    pub fn try_borrow(&self) -> Result<&V, NodeInUse> {
        self.value.as_ref().ok_or(NodeInUse)
    }

    #[inline]
    #[track_caller]
    #[allow(clippy::should_implement_trait)]
    pub fn borrow(&self) -> &V {
        match self.try_borrow() {
            Ok(b) => b,
            Err(_) => panic!("node is currently in use"),
        }
    }

    #[inline]
    pub fn try_borrow_mut(&mut self) -> Result<&mut V, NodeInUse> {
        self.value.as_mut().ok_or(NodeInUse)
    }

    #[inline]
    #[track_caller]
    #[allow(clippy::should_implement_trait)]
    pub fn borrow_mut(&mut self) -> &mut V {
        match self.try_borrow_mut() {
            Ok(b) => b,
            Err(_) => panic!("node is currently in use"),
        }
    }

    pub fn take(&mut self) -> Result<V, NodeInUse> {
        self.value.take().ok_or(NodeInUse)
    }
}

impl<K, V> TreeNode<K, V>
where
    K: slotmap::Key,
{
    pub fn parent(&self) -> Option<&K> {
        self.parent.as_ref()
    }

    pub fn children(&self) -> &[K] {
        &self.children
    }
}

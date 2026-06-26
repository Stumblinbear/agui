use crate::semantics::{SemanticsConfig, SemanticsNodeId};

/// One node in a [`SemanticsTree`] snapshot: the accessibility properties of a contributing render object
/// after its absorbed descendants are folded in, its stable id, and its semantic children.
#[derive(Debug, Clone)]
pub struct SemanticsNode {
    pub id: SemanticsNodeId,
    pub config: SemanticsConfig,
    pub children: Vec<SemanticsNode>,
}

impl SemanticsNode {
    /// This node's accessible name: the name computed for it when the tree was assembled, or `None` when it
    /// has none.
    pub fn accessible_name(&self) -> Option<String> {
        self.config.node.label().map(ToOwned::to_owned)
    }

    /// Visits this node and every descendant, parents before children.
    pub fn visit(&self, f: &mut impl FnMut(&SemanticsNode)) {
        f(self);

        for child in &self.children {
            child.visit(f);
        }
    }
}

/// A snapshot of the semantics tree, assembled on demand from a walk of the render tree.
#[derive(Debug, Clone, Default)]
pub struct SemanticsTree {
    roots: Vec<SemanticsNode>,
}

impl SemanticsTree {
    #[must_use]
    pub fn new(roots: Vec<SemanticsNode>) -> Self {
        Self { roots }
    }

    /// The top-level nodes. There is more than one only when the root subtree is transparent above several
    /// contributing nodes.
    #[must_use]
    pub fn roots(&self) -> &[SemanticsNode] {
        &self.roots
    }

    /// The first node, in walk order, that satisfies `predicate`.
    pub fn find(
        &self,
        mut predicate: impl FnMut(&SemanticsNode) -> bool,
    ) -> Option<&SemanticsNode> {
        fn search<'a>(
            node: &'a SemanticsNode,
            predicate: &mut impl FnMut(&SemanticsNode) -> bool,
        ) -> Option<&'a SemanticsNode> {
            if predicate(node) {
                return Some(node);
            }

            node.children
                .iter()
                .find_map(|child| search(child, predicate))
        }

        self.roots
            .iter()
            .find_map(|node| search(node, &mut predicate))
    }

    /// The first node whose [`accessible_name`](SemanticsNode::accessible_name) equals `name`.
    #[must_use]
    pub fn find_by_name(&self, name: &str) -> Option<&SemanticsNode> {
        self.find(|node| node.accessible_name().as_deref() == Some(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use accesskit::Role;

    #[test]
    fn find_by_name_uses_the_computed_name() {
        let mut config = SemanticsConfig::new(Role::Button);
        config.node.set_label("Submit");

        let tree = SemanticsTree::new(vec![SemanticsNode {
            id: SemanticsNodeId(0),
            config,
            children: Vec::new(),
        }]);

        assert_eq!(
            tree.find_by_name("Submit")
                .map(|node| node.config.node.role()),
            Some(Role::Button),
        );
        assert!(tree.find_by_name("missing").is_none());
    }
}

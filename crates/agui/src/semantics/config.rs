use accesskit::Role;

/// A stable identity for one semantics node, minted from a monotonic counter the first time a render object
/// emits a node and kept by that render object thereafter. It does not change when the node's siblings
/// reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemanticsNodeId(pub u64);

/// What one render object contributes to the semantics tree: an [`accesskit::Node`] of accessibility
/// properties, plus the flags that control how agui folds it into the tree.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default)]
pub struct SemanticsConfig {
    /// The node's accessibility properties: role, label, value, actions, and state.
    pub node: accesskit::Node,

    /// Force this contribution to be its own node even when an enclosing node would otherwise absorb it.
    pub is_boundary: bool,

    /// Keep this node's descendants as their own nodes rather than absorbing them into this one.
    pub explicit_children: bool,

    /// Fold this node's whole subtree into one node, as a button absorbing its label.
    pub merge_descendants: bool,

    /// Drop the semantics of nodes painted before this one, as a modal barrier does.
    pub blocks_previous: bool,

    /// Drop this node and its subtree, as for purely decorative content.
    pub excluded: bool,
}

impl SemanticsConfig {
    /// A config for a node of `role`, with no other properties set.
    #[must_use]
    pub fn new(role: Role) -> Self {
        Self {
            node: accesskit::Node::new(role),
            ..Default::default()
        }
    }

    pub fn boundary(self) -> SemanticsConfig {
        Self {
            is_boundary: true,
            ..self
        }
    }

    pub fn explicit_children(self) -> SemanticsConfig {
        Self {
            explicit_children: true,
            ..self
        }
    }

    pub fn merge_descendants(self) -> SemanticsConfig {
        Self {
            merge_descendants: true,
            ..self
        }
    }

    pub fn blocks_previous(self) -> SemanticsConfig {
        Self {
            blocks_previous: true,
            ..self
        }
    }

    pub fn excluded(self) -> SemanticsConfig {
        Self {
            excluded: true,
            ..self
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("node is currently in use")]
pub struct NodeInUse;

#[derive(Debug, thiserror::Error)]
pub enum SwapSiblingsError {
    #[error("parent node does not exist")]
    ParentNotFound,

    #[error("sibling node does not exist")]
    SiblingNotFound,

    #[error("nodes are not children of the same parent")]
    NotAChild,
}

#[derive(Debug, thiserror::Error)]
pub enum ReparentError {
    #[error("parent node does not exist")]
    NewParentNotFound,

    #[error("sibling node does not exist")]
    NodeNotFound,

    #[error("node is already a child of the new parent")]
    Unmoved,
}

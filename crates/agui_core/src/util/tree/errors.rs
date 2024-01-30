#[derive(Debug, thiserror::Error)]
#[error("node is currently in use")]
pub struct NodeInUse;

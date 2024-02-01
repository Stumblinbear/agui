#[derive(Debug, thiserror::Error)]
pub enum SpawnError<K> {
    #[error("the tree is in an invalid state")]
    Broken,

    #[error("parent node not found: {0:?}")]
    NotFound(K),
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError<K> {
    #[error("the tree is in an invalid state")]
    Broken,

    #[error("node not found: {0:?}")]
    NotFound(K),
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateChildrenError<K> {
    #[error("the tree is in an invalid state")]
    Broken,

    #[error("node not found: {0:?}")]
    NotFound(K),

    #[error("cannot update {0:?} as it is currently in use")]
    InUse(K),

    #[error("failed to spawn child node: {0}")]
    SpawnChild(#[from] SpawnError<K>),
}

#[derive(Debug, thiserror::Error)]
pub enum RemoveError<K> {
    #[error("node not found")]
    NotFound(K),

    #[error("could not call unmount on {0:?} as it is currently in use")]
    InUse(K),
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError<K> {
    #[error("the tree is in an invalid state")]
    Broken,

    #[error("an node that was due to be rebuilt was missing from the tree")]
    Missing(K),

    #[error("failed to spawn node: {0}")]
    Spawn(#[from] SpawnError<K>),

    #[error("unable to update {0:?} as it is currently in use")]
    InUse(K),
}

impl<K> From<UpdateChildrenError<K>> for BuildError<K> {
    fn from(error: UpdateChildrenError<K>) -> Self {
        match error {
            UpdateChildrenError::Broken => BuildError::Broken,
            UpdateChildrenError::NotFound(id) => BuildError::Missing(id),
            UpdateChildrenError::InUse(id) => BuildError::InUse(id),
            UpdateChildrenError::SpawnChild(err) => BuildError::Spawn(err),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SpawnError<K> {
    #[error("the tree is in an invalid state")]
    Broken,

    #[error("failed to mount: {0:?}")]
    Mount(#[from] MountError<K>),
}

#[derive(Debug, thiserror::Error)]
pub enum MountError<K> {
    #[error("parent node not found: {0:?}")]
    ParentNotFound(K),
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateChildrenError<K> {
    #[error("the tree is in an invalid state")]
    Broken,

    #[error("parent node not found: {0:?}")]
    ParentNotFound(K),

    #[error("cannot update {0:?} as it is currently in use")]
    InUse(K),
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

    #[error("node not found: {0:?}")]
    NotFound(K),

    #[error("unable to update {0:?} as it is currently in use")]
    InUse(K),
}

#[derive(Debug, thiserror::Error)]
pub enum SpawnAndInflateError<K> {
    #[error("the tree is in an invalid state")]
    Broken,

    #[error("failed to mount root: {0:?}")]
    Mount(#[from] MountError<K>),

    #[error("failed to inflate: {0:?}")]
    Inflate(#[from] BuildError<K>),
}

impl<K> From<UpdateChildrenError<K>> for BuildError<K> {
    fn from(error: UpdateChildrenError<K>) -> Self {
        match error {
            UpdateChildrenError::Broken => BuildError::Broken,
            UpdateChildrenError::ParentNotFound(id) => BuildError::NotFound(id),
            UpdateChildrenError::InUse(id) => BuildError::InUse(id),
        }
    }
}

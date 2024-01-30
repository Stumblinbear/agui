use crate::element::ElementId;

#[derive(Debug, thiserror::Error)]
pub enum SpawnElementError {
    #[error("the tree is in an invalid state")]
    Broken,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateElementError {
    #[error("the tree is in an invalid state")]
    Broken,

    #[error("element not found")]
    NotFound,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateElementChildrenError {
    #[error("the tree is in an invalid state")]
    Broken,

    #[error("element not found")]
    NotFound(ElementId),

    #[error("cannot update {0:?} as it is currently in use")]
    InUse(ElementId),

    #[error("failed to spawn child: {0}")]
    SpawnChild(#[from] SpawnElementError),
}

#[derive(Debug, thiserror::Error)]
pub enum RemoveElementError {
    #[error("element not found")]
    NotFound(ElementId),

    #[error("could not call unmount on {0:?} as it is currently in use")]
    InUse(ElementId),
}

#[derive(Debug, thiserror::Error)]
pub enum InflateError {
    #[error("the tree is in an invalid state")]
    Broken,

    #[error("an element that was due to be rebuilt was missing from the tree")]
    Missing(ElementId),

    #[error("failed to spawn element: {0}")]
    Spawn(SpawnElementError),

    #[error("unable to update {0:?} as it is currently in use")]
    InUse(ElementId),
}

impl From<SpawnElementError> for InflateError {
    fn from(error: SpawnElementError) -> Self {
        match error {
            SpawnElementError::Broken => InflateError::Broken,
        }
    }
}

impl From<UpdateElementChildrenError> for InflateError {
    fn from(error: UpdateElementChildrenError) -> Self {
        match error {
            UpdateElementChildrenError::Broken => InflateError::Broken,
            UpdateElementChildrenError::NotFound(id) => InflateError::Missing(id),
            UpdateElementChildrenError::InUse(id) => InflateError::InUse(id),
            UpdateElementChildrenError::SpawnChild(err) => InflateError::Spawn(err),
        }
    }
}

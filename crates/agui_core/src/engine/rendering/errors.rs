#[derive(Debug, thiserror::Error)]
pub enum RemoveError<K> {
    #[error("node not found")]
    NotFound(K),

    #[error("could not call unmount on {0:?} as it is currently in use")]
    InUse(K),
}

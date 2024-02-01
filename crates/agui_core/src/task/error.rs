#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("executor was shutdown")]
    Shutdown,

    #[error("no scheduler bound")]
    NoScheduler(NoScheduler),
}

impl TaskError {
    pub fn no_scheduler() -> Self {
        Self::NoScheduler(NoScheduler {})
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct NoScheduler {}

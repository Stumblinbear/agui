use std::pin::Pin;

use crate::task::TaskHandle;

pub trait TaskScheduler {
    /// Spawn an async task to perform the given work on the current thread. These tasks will be driven
    /// on the same thread that the UI is running on
    fn spawn_local(
        &self,
        func: impl FnOnce() -> Pin<Box<dyn Future<Output = ()>>>,
    ) -> Result<TaskHandle, Box<dyn std::error::Error>>;

    /// Spawn an async taskto perform the given work on a background thread.
    fn spawn_threaded(
        &self,
        func: impl FnOnce() -> Pin<Box<dyn Future<Output = ()>>>,
    ) -> Result<TaskHandle, Box<dyn std::error::Error>>;
}

mod local;
mod shared;
#[cfg(feature = "multi-threaded")]
mod threaded;

use futures::prelude::Future;
pub use local::LocalEngineExecutor;
#[cfg(feature = "multi-threaded")]
pub use threaded::ThreadedEngineExecutor;

pub trait EngineExecutor {
    /// Updates the engine until the tree has settled.
    ///
    /// This does not execute async tasks.
    fn update(&mut self);

    /// Updates the engine until the tree has settled and no more progress can
    /// be made in any async tasks.
    fn run_until_stalled(&mut self);

    fn run_until<Fut, Out>(self, fut: Fut) -> Out
    where
        Fut: Future<Output = Out>;

    fn run(self);
}

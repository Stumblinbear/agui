mod local;
mod threaded;

pub use local::LocalEngineExecutor;
pub use threaded::ThreadedEngineExecutor;

pub trait EngineExecutor {
    /// Updates the engine until the tree has settled.
    ///
    /// This does not execute async tasks.
    fn update(&mut self);

    /// Updates the engine until the tree has settled and no more progress can
    /// be made in any async tasks.
    fn run_until_stalled(&mut self);

    fn run(self);
}

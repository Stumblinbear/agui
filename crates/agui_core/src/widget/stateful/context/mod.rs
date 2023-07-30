mod build;
mod callback;

pub use build::*;
pub use callback::*;

pub trait ContextWidgetStateMut<'ctx, S> {
    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut S) + 'static;
}

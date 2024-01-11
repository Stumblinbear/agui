mod build;
mod callback;
pub(crate) mod func;

pub use build::*;
pub use callback::*;

pub trait ContextWidgetStateMut<'ctx, S>
where
    S: ?Sized,
{
    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut S);
}

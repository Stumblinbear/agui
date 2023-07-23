mod inheritance;
mod instance;

pub(crate) use inheritance::*;
pub use instance::*;

pub trait InheritedWidget: Sized + 'static {
    #[allow(unused_variables)]
    fn should_notify(&self, old_widget: &Self) -> bool {
        true
    }
}

pub trait ContextInheritedMut {
    fn depend_on_inherited_widget<I>(&mut self) -> Option<&mut I>
    where
        I: InheritedWidget + 'static;
}

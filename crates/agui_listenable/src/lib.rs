mod emitter;
mod notifier;
mod value;

pub use emitter::*;
pub use notifier::*;
pub use value::*;

pub trait Listenable {
    type Handle;

    fn notify_listeners(&self);

    fn add_listener(&self, func: impl Fn() + 'static) -> Self::Handle;
}

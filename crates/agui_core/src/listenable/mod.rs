mod bus;
mod emitter;
mod event;
mod notifier;
mod value;

pub use bus::*;
pub use emitter::*;
pub use event::*;
pub use notifier::*;
pub use value::*;

pub trait Listenable {
    type Handle;

    fn notify_listeners(&self);

    #[must_use]
    fn add_listener(&self, func: impl Fn() + 'static) -> Self::Handle;
}

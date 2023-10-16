use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Weak},
};

use super::Listenable;

#[derive(Clone, Default)]
#[allow(clippy::type_complexity)]
pub struct Notifier {
    listeners: Rc<RefCell<Vec<Weak<dyn Fn()>>>>,
}

impl Notifier {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Listenable for Notifier {
    type Handle = NotifierHandle;

    fn notify_listeners(&self) {
        self.listeners.borrow_mut().retain(|handle| {
            if let Some(handle) = handle.upgrade() {
                (handle)();
                true
            } else {
                false
            }
        });
    }

    fn add_listener(&self, func: impl Fn() + 'static) -> Self::Handle {
        let func = Arc::new(func) as Arc<dyn Fn()>;

        self.listeners.borrow_mut().push(Arc::downgrade(&func));

        NotifierHandle { _guard: func }
    }
}

#[derive(Clone)]
pub struct NotifierHandle {
    _guard: Arc<dyn Fn()>,
}

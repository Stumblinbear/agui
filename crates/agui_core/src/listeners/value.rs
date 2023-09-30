use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use super::{Listenable, Notifier, NotifierHandle};

#[derive(Clone, Default)]
pub struct ValueNotifier<T> {
    value: Rc<RefCell<T>>,

    notifier: Notifier,
}

impl<T> ValueNotifier<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),

            notifier: Notifier::default(),
        }
    }

    pub fn get(&self) -> Ref<T> {
        self.value.borrow()
    }

    pub fn set(&self, value: T) {
        *self.value.borrow_mut() = value;

        self.notify_listeners();
    }
}

impl<T> Listenable for ValueNotifier<T> {
    type Handle = NotifierHandle;

    fn notify_listeners(&self) {
        self.notifier.notify_listeners()
    }

    fn add_listener(&self, func: impl Fn() + 'static) -> NotifierHandle {
        self.notifier.add_listener(func)
    }
}

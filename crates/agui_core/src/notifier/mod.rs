use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use fnv::FnvHashMap;

type Subscribers<T> = Rc<RefCell<FnvHashMap<usize, Box<dyn Listener<T>>>>>;

pub trait Listener<T> {
    fn on_notified(&self, value: &T);
}

pub struct Notifier<T> {
    last_listener_id: RefCell<usize>,

    subscribers: Subscribers<T>,
}

impl<T> Default for Notifier<T> {
    fn default() -> Self {
        Self {
            last_listener_id: RefCell::default(),

            subscribers: Rc::default(),
        }
    }
}

impl<T> Notifier<T>
where
    T: 'static,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn notify(&self, value: &T) {
        let subscribers = self.subscribers.borrow();

        for func in subscribers.values() {
            func.on_notified(value);
        }
    }

    pub fn add_listener(&self, func: impl Fn(&T) + 'static) -> ListenerHandle<T> {
        let mut last_listener_id = self.last_listener_id.borrow_mut();

        let listener_id = last_listener_id.wrapping_add(1);

        *last_listener_id = listener_id;

        self.subscribers.borrow_mut().insert(
            listener_id,
            Box::new(ListenerFunc {
                phantom: PhantomData,

                func: Box::new(func),
            }),
        );

        ListenerHandle {
            listener_id,
            subscribers: Rc::clone(&self.subscribers),
        }
    }
}

struct ListenerFunc<F, T> {
    phantom: PhantomData<T>,

    func: Box<F>,
}

impl<F, T> Listener<T> for ListenerFunc<F, T>
where
    F: Fn(&T),
{
    fn on_notified(&self, value: &T) {
        (self.func)(value)
    }
}

pub struct ListenerHandle<T> {
    listener_id: usize,
    subscribers: Subscribers<T>,
}

impl<T> Drop for ListenerHandle<T> {
    fn drop(&mut self) {
        self.subscribers.borrow_mut().remove(&self.listener_id);
    }
}

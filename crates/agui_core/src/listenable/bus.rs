use std::{any::TypeId, cell::RefCell, rc::Rc};

use crate::{
    listenable::{Event, EventEmitter, EventEmitterHandle},
    unit::AsAny,
    util::map::TypeMap,
};

#[derive(Default)]
#[allow(clippy::type_complexity)]
pub struct EventBus {
    emitters: Rc<RefCell<TypeMap<Box<dyn EventBusEmitter>>>>,
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            emitters: Rc::clone(&self.emitters),
        }
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn emit<T: Event>(&self, value: &T) {
        if let Some(emitter) = self.emitters.borrow().get(&TypeId::of::<T>()) {
            let emitter = (**emitter)
                .as_any()
                .downcast_ref::<EventEmitter<T>>()
                .expect("failed to downcast event bus emitter");

            emitter.emit(value);
        }
    }

    #[must_use]
    pub fn add_listener<T: Event>(&self, func: impl Fn(&T) + 'static) -> EventEmitterHandle<T> {
        let mut emitters = self.emitters.borrow_mut();

        if let Some(emitter) = emitters.get(&TypeId::of::<T>()) {
            let emitter = (**emitter)
                .as_any()
                .downcast_ref::<EventEmitter<T>>()
                .expect("failed to downcast event bus emitter");

            emitter.add_listener(func)
        } else {
            let emitter = Box::new(EventEmitter::<T>::new());

            let handle = emitter.add_listener(func);

            emitters.insert(TypeId::of::<T>(), emitter);

            handle
        }
    }
}

trait EventBusEmitter: AsAny {}

impl<T: AsAny> EventBusEmitter for EventEmitter<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestEvent;

    impl Event for TestEvent {}

    struct OtherTestEvent;

    impl Event for OtherTestEvent {}

    #[test]
    fn calls_event() {
        let bus = EventBus::new();

        let did_call = Rc::new(RefCell::new(false));

        let _handle = bus.add_listener::<TestEvent>({
            let did_call = Rc::clone(&did_call);

            move |_| {
                *did_call.borrow_mut() = true;
            }
        });

        bus.emit(&TestEvent);

        assert!(*did_call.borrow());
    }

    #[test]
    fn calls_all_events() {
        let bus = EventBus::new();

        let num_calls = Rc::new(RefCell::new(0));

        let _handle1 = bus.add_listener::<TestEvent>({
            let num_calls = Rc::clone(&num_calls);

            move |_| {
                *num_calls.borrow_mut() += 1;
            }
        });

        let _handle2 = bus.add_listener::<TestEvent>({
            let num_calls = Rc::clone(&num_calls);

            move |_| {
                *num_calls.borrow_mut() += 1;
            }
        });

        bus.emit(&TestEvent);

        assert_eq!(*num_calls.borrow(), 2);
    }

    #[test]
    fn multiple_event_types() {
        let bus = EventBus::new();

        let test_calls = Rc::new(RefCell::new(0));
        let other_calls = Rc::new(RefCell::new(0));

        let _handle1 = bus.add_listener::<TestEvent>({
            let test_calls = Rc::clone(&test_calls);

            move |_| {
                *test_calls.borrow_mut() += 1;
            }
        });

        let _handle2 = bus.add_listener::<OtherTestEvent>({
            let other_calls = Rc::clone(&other_calls);

            move |_| {
                *other_calls.borrow_mut() += 1;
            }
        });

        bus.emit(&TestEvent);
        bus.emit(&TestEvent);
        bus.emit(&OtherTestEvent);

        assert_eq!(*test_calls.borrow(), 2);
        assert_eq!(*other_calls.borrow(), 1);
    }
}

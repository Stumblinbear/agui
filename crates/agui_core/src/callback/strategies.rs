use std::any::Any;

use crate::callback::CallbackId;

pub trait CallbackStrategy {
    fn call_unchecked(&self, callback_id: CallbackId, arg: Box<dyn Any + Send>);
}
#[cfg(any(test, feature = "mocks"))]
pub mod mocks {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Default, Clone)]
    pub struct MockCallbackStratgy {
        #[allow(clippy::type_complexity)]
        pub calls: Arc<Mutex<Vec<(CallbackId, Box<dyn Any + Send>)>>>,
    }

    impl CallbackStrategy for MockCallbackStratgy {
        fn call_unchecked(&self, callback_id: CallbackId, arg: Box<dyn Any + Send>) {
            self.calls
                .lock()
                .expect("callback tracker poisoned")
                .push((callback_id, arg));
        }
    }
}

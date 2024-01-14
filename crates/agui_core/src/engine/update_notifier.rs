use std::sync::Arc;

use parking_lot::{Condvar, Mutex};

#[derive(Clone)]
pub struct UpdateNotifier {
    notifier: Arc<(Mutex<bool>, Condvar)>,
}

impl UpdateNotifier {
    pub(crate) fn new() -> Self {
        Self {
            notifier: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    pub fn notify(&self) {
        let (mutex, cond) = &*self.notifier;
        let mut guard = mutex.lock();
        *guard = true;
        cond.notify_one();
    }

    pub fn wait(&self) {
        let (mutex, cond) = &*self.notifier;
        let mut guard = mutex.lock();
        while !*guard {
            cond.wait(&mut guard);
        }
        *guard = false;
    }
}

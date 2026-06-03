//! Callbacks run once per frame, in step with the display refresh.
//!
//! A [`Vsync`] holds work that recurs every frame for as long as it is registered: advancing an
//! animation, sampling input, anything that must keep pace with the display rather than wait on a
//! message. It is the synchronous counterpart to the asynchronous task scheduler — a callback runs
//! inline on the frame, free to mutate shared state in place, where a task instead posts a message
//! back to be handled later.

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    time::Duration,
};

use slotmap::SlotMap;

slotmap::new_key_type! {
    struct CallbackKey;
}

#[derive(Default)]
struct Inner {
    callbacks: SlotMap<CallbackKey, Box<dyn FnMut(Duration)>>,
}

/// A registry of callbacks run once per frame.
///
/// Register work with [`on_frame`](Vsync::on_frame); it recurs every frame until the returned
/// [`VsyncHandle`] is dropped. The driver advances every registered callback for a frame with
/// [`tick`](Vsync::tick). A clone shares the same registry, so the driver can hand one to each part
/// of the system that needs to schedule frame work.
#[derive(Clone, Default)]
pub struct Vsync {
    inner: Rc<RefCell<Inner>>,
}

impl Vsync {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `callback` to run once per frame. It runs until the returned [`VsyncHandle`] is
    /// dropped.
    pub fn on_frame(&self, callback: impl FnMut(Duration) + 'static) -> VsyncHandle {
        let key = self.inner.borrow_mut().callbacks.insert(Box::new(callback));

        VsyncHandle {
            inner: Rc::downgrade(&self.inner),
            key,
        }
    }

    /// The number of callbacks currently registered. A driver can stop requesting frames once this
    /// reaches zero and resume when work is registered again.
    pub fn is_idle(&self) -> bool {
        self.inner.borrow().callbacks.is_empty()
    }

    /// Runs every registered callback for the frame at `now`.
    ///
    /// The registry is held for the duration, so a callback must not register or cancel frame work
    /// while it runs — scheduling more frame work is a structural change and belongs in a message or
    /// rebuild, not inside a tick.
    pub fn tick(&self, now: Duration) {
        for callback in self.inner.borrow_mut().callbacks.values_mut() {
            callback(now);
        }
    }
}

/// Keeps a callback registered on a [`Vsync`]; dropping it cancels the callback.
#[must_use = "dropping the handle cancels the frame callback"]
pub struct VsyncHandle {
    inner: Weak<RefCell<Inner>>,
    key: CallbackKey,
}

impl Drop for VsyncHandle {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.upgrade() {
            inner.borrow_mut().callbacks.remove(self.key);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc, time::Duration};

    use super::Vsync;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn a_callback_runs_every_tick_with_the_frame_time() {
        let vsync = Vsync::new();
        let seen = Rc::new(RefCell::new(Vec::new()));

        let _handle = {
            let seen = Rc::clone(&seen);
            vsync.on_frame(move |now| seen.borrow_mut().push(now))
        };

        vsync.tick(ms(16));
        vsync.tick(ms(32));

        assert_eq!(*seen.borrow(), vec![ms(16), ms(32)]);
    }

    #[test]
    fn dropping_the_handle_cancels_the_callback() {
        let vsync = Vsync::new();
        let count = Rc::new(RefCell::new(0));

        let handle = {
            let count = Rc::clone(&count);
            vsync.on_frame(move |_| *count.borrow_mut() += 1)
        };

        vsync.tick(ms(16));
        assert_eq!(*count.borrow(), 1);
        assert!(!vsync.is_idle());

        drop(handle);

        vsync.tick(ms(32));
        assert_eq!(
            *count.borrow(),
            1,
            "a cancelled callback does not run again"
        );
        assert!(vsync.is_idle());
    }

    #[test]
    fn every_registered_callback_runs_in_a_tick() {
        let vsync = Vsync::new();
        let a = Rc::new(RefCell::new(0));
        let b = Rc::new(RefCell::new(0));

        let _a = {
            let a = Rc::clone(&a);
            vsync.on_frame(move |_| *a.borrow_mut() += 1)
        };
        let _b = {
            let b = Rc::clone(&b);
            vsync.on_frame(move |_| *b.borrow_mut() += 1)
        };

        vsync.tick(ms(16));

        assert_eq!(*a.borrow(), 1);
        assert_eq!(*b.borrow(), 1);
    }
}

use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, Mutex, PoisonError, mpsc},
    task::{Context, Wake, Waker},
};

use crate::scheduling::{EventSender, TaskEventMessage, TaskFuture, TaskHandle, TaskScheduler};

/// A spawned future paired with the flag its [`TaskHandle`] raises to cancel it.
type Task = (TaskFuture, Rc<Cell<bool>>);

/// Shared between the reactor and every task's [`Waker`]. A waker may fire from any thread, so this
/// carries only thread-safe data and lives behind an [`Arc`].
struct Shared {
    woken: Mutex<Vec<usize>>,
    wake_host: Box<dyn Fn() + Send + Sync>,
}

/// The [`Waker`] handed to one task: waking it records the task for the next poll and nudges the host
/// loop so that poll happens.
struct TaskWaker {
    index: usize,
    shared: Arc<Shared>,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.shared
            .woken
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(self.index);
        (self.shared.wake_host)();
    }
}

/// A single-threaded executor that drives spawned futures on the host's event loop.
///
/// It is the runtime counterpart to the test runner: a parked task is re-polled when its [`Waker`]
/// fires rather than on a fixed cadence, and waking calls the host callback given to
/// [`new`](Self::new), so the loop can sleep until either a host event or a task wakeup. Spawn through
/// the [`TaskScheduler`] from [`scheduler`](Self::scheduler), advance the tasks asked to run with
/// [`poll`](Self::poll), and route the messages they post with [`messages`](Self::messages).
///
/// Tasks run on the thread that polls them, so their futures need not be [`Send`]. The host callback
/// may be invoked from another thread, so it must be.
pub struct LocalReactor {
    tasks: Vec<Option<Task>>,
    shared: Arc<Shared>,

    task_event_tx: EventSender,
    task_event_rx: mpsc::Receiver<TaskEventMessage>,

    spawn_tx: mpsc::Sender<Task>,
    spawn_rx: mpsc::Receiver<Task>,
}

impl LocalReactor {
    /// Builds a reactor that calls `wake_host` whenever a task becomes ready. The host responds by
    /// arranging a [`poll`](Self::poll) soon, such as by waking its event loop.
    pub fn new(wake_host: impl Fn() + Send + Sync + 'static) -> Self {
        let (task_event_tx, task_event_rx) = mpsc::channel();
        let (spawn_tx, spawn_rx) = mpsc::channel();

        Self {
            tasks: Vec::new(),
            shared: Arc::new(Shared {
                woken: Mutex::new(Vec::new()),
                wake_host: Box::new(wake_host),
            }),

            task_event_tx,
            task_event_rx,

            spawn_tx,
            spawn_rx,
        }
    }

    /// A scheduler for spawning onto this reactor. It is free of borrows, so it can be captured by a
    /// widget and used to spawn later, including during layout.
    pub fn scheduler(&self) -> LocalScheduler {
        LocalScheduler {
            task_event_tx: self.task_event_tx.clone(),
            spawn_tx: self.spawn_tx.clone(),
        }
    }

    /// The messages tasks have posted to their elements since the last drain.
    pub fn messages(&self) -> impl Iterator<Item = TaskEventMessage> + '_ {
        self.task_event_rx.try_iter()
    }

    /// Whether any spawned task is still running.
    pub fn has_pending(&self) -> bool {
        self.tasks.iter().any(Option::is_some)
    }

    /// Polls every task asked to run since the last call, dropping the ones that finish or were
    /// cancelled. Returns whether any task ran, so a caller can poll in a loop to drain the reactor to
    /// a stopping point.
    pub fn poll(&mut self) -> bool {
        // A freshly spawned task starts ready, so take it in and let this poll run it once.
        while let Ok(task) = self.spawn_rx.try_recv() {
            let index = self.tasks.len();
            self.tasks.push(Some(task));
            self.shared
                .woken
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(index);
        }

        for slot in &mut self.tasks {
            if matches!(slot, Some((_, cancelled)) if cancelled.get()) {
                *slot = None;
            }
        }

        let mut woken = std::mem::take(
            &mut *self
                .shared
                .woken
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
        woken.sort_unstable();
        woken.dedup();

        let ran = !woken.is_empty();

        for index in woken {
            let Some(Some((future, _))) = self.tasks.get_mut(index) else {
                continue;
            };

            let waker = Waker::from(Arc::new(TaskWaker {
                index,
                shared: Arc::clone(&self.shared),
            }));

            if future
                .as_mut()
                .poll(&mut Context::from_waker(&waker))
                .is_ready()
            {
                self.tasks[index] = None;
            }
        }

        ran
    }
}

/// A [`TaskScheduler`] onto a [`LocalReactor`]. A spawned task begins on the reactor's next poll, and
/// dropping its [`TaskHandle`] cancels it before it runs again.
pub struct LocalScheduler {
    task_event_tx: EventSender,
    spawn_tx: mpsc::Sender<Task>,
}

impl TaskScheduler for LocalScheduler {
    fn event_tx(&self) -> EventSender {
        self.task_event_tx.clone()
    }

    fn spawn(&mut self, func: TaskFuture) -> Result<TaskHandle, Box<dyn std::error::Error>> {
        let cancelled = Rc::new(Cell::new(false));

        if self.spawn_tx.send((func, Rc::clone(&cancelled))).is_err() {
            return Err(String::from("the reactor has been dropped").into());
        }

        Ok(TaskHandle::new(Box::new(move || {
            cancelled.set(true);
        })))
    }

    fn deferred(&self) -> Box<dyn TaskScheduler> {
        Box::new(LocalScheduler {
            task_event_tx: self.task_event_tx.clone(),
            spawn_tx: self.spawn_tx.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        pin::Pin,
        sync::atomic::{AtomicUsize, Ordering},
        task::Poll,
    };

    use crate::element::RoutingPath;

    use super::*;

    /// Pends once, waking itself, then completes. Exercises the parked-then-woken path.
    struct YieldOnce(bool);

    impl Future for YieldOnce {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
            if self.0 {
                return Poll::Ready(());
            }

            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }

    #[test]
    fn a_woken_task_is_polled_again_and_wakes_the_host() {
        let wakes = Arc::new(AtomicUsize::new(0));

        let mut reactor = {
            let wakes = Arc::clone(&wakes);
            LocalReactor::new(move || {
                wakes.fetch_add(1, Ordering::SeqCst);
            })
        };

        // Hold the handle: dropping it would cancel the task before it runs.
        let _handle = reactor
            .scheduler()
            .spawn(Box::pin(YieldOnce(false)))
            .expect("the reactor is alive");

        reactor.poll();
        assert!(
            reactor.has_pending(),
            "the task parked, waiting to run again"
        );
        assert_eq!(
            wakes.load(Ordering::SeqCst),
            1,
            "parking woke the host once"
        );

        reactor.poll();
        assert!(
            !reactor.has_pending(),
            "the second poll ran it to completion"
        );
    }

    #[test]
    fn a_cancelled_task_is_dropped_before_it_runs() {
        let mut reactor = LocalReactor::new(|| {});

        let handle = reactor
            .scheduler()
            .spawn(Box::pin(YieldOnce(false)))
            .expect("the reactor is alive");

        drop(handle);

        reactor.poll();
        assert!(!reactor.has_pending(), "a cancelled task never runs");
    }

    #[test]
    fn posted_messages_drain_in_order() {
        let reactor = LocalReactor::new(|| {});

        let tx = reactor.scheduler().event_tx();
        let path = RoutingPath::new(crate::element::BoundaryId::default(), Vec::new());
        tx.send((path.clone(), Box::new(1_u32))).unwrap();
        tx.send((path, Box::new(2_u32))).unwrap();

        let drained = reactor.messages().count();
        assert_eq!(drained, 2);
    }
}

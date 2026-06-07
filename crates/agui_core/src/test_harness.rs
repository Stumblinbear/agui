use std::{
    cell::Cell,
    rc::Rc,
    sync::mpsc,
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

use crate::{
    context::UpdateCtx,
    element::BuildScope,
    provide::ProvideScope,
    scheduling::{EventSender, TaskEventMessage, TaskFuture, TaskHandle, TaskScheduler},
};

enum TaskRunnerEvent {
    Dropped(usize),
}

type DeferredTask = (TaskFuture, Rc<Cell<bool>>);

/// Test scheduler that drives spawned tasks to completion synchronously, so a test can mount and
/// then drain the event channel deterministically.
pub struct TestTaskRunner {
    task_event_rx: mpsc::Receiver<TaskEventMessage>,
    task_event_tx: EventSender,

    spawned_tasks: Vec<Option<TaskFuture>>,

    runner_event_rx: mpsc::Receiver<TaskRunnerEvent>,
    runner_event_tx: mpsc::Sender<TaskRunnerEvent>,

    deferred_rx: mpsc::Receiver<DeferredTask>,
    deferred_tx: mpsc::Sender<DeferredTask>,
    deferred_tasks: Vec<DeferredTask>,
}

impl TestTaskRunner {
    pub fn new() -> Self {
        let (task_event_tx, task_event_rx) = mpsc::channel();
        let (runner_event_tx, runner_event_rx) = mpsc::channel();
        let (deferred_tx, deferred_rx) = mpsc::channel();

        Self {
            task_event_rx,
            task_event_tx,

            spawned_tasks: Vec::new(),

            runner_event_rx,
            runner_event_tx,

            deferred_rx,
            deferred_tx,
            deferred_tasks: Vec::new(),
        }
    }

    pub fn scheduler(&mut self) -> TestTaskScheduler<'_> {
        TestTaskScheduler {
            task_event_tx: self.task_event_tx.clone(),
            spawned_tasks: &mut self.spawned_tasks,
            runner_event_tx: &self.runner_event_tx,
            deferred_tx: self.deferred_tx.clone(),
        }
    }

    pub fn messages(&self) -> impl Iterator<Item = TaskEventMessage> + '_ {
        self.task_event_rx.try_iter()
    }

    /// Poll every live spawned task once, dropping any that complete or whose handle was dropped.
    pub fn poll(&mut self) {
        self.reap_dropped();

        while let Ok(task) = self.deferred_rx.try_recv() {
            self.deferred_tasks.push(task);
        }

        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);

        for slot in &mut self.spawned_tasks {
            if let Some(future) = slot.as_mut()
                && future.as_mut().poll(&mut cx).is_ready()
            {
                *slot = None;
            }
        }

        self.deferred_tasks.retain_mut(|(future, cancelled)| {
            !cancelled.get() && future.as_mut().poll(&mut cx).is_pending()
        });
    }

    /// Poll repeatedly until no live task remains. A task that never completes will spin forever;
    /// tests are expected to spawn tasks that finish.
    pub fn run_to_completion(&mut self) {
        while {
            self.poll();
            self.spawned_tasks.iter().any(Option::is_some) || !self.deferred_tasks.is_empty()
        } {}
    }

    /// Drop the futures of any tasks whose [`TaskHandle`] has been dropped.
    fn reap_dropped(&mut self) {
        while let Ok(TaskRunnerEvent::Dropped(id)) = self.runner_event_rx.try_recv() {
            if let Some(slot) = self.spawned_tasks.get_mut(id) {
                *slot = None;
            }
        }
    }
}

impl Default for TestTaskRunner {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TestTaskScheduler<'a> {
    task_event_tx: EventSender,

    spawned_tasks: &'a mut Vec<Option<TaskFuture>>,
    runner_event_tx: &'a mpsc::Sender<TaskRunnerEvent>,
    deferred_tx: mpsc::Sender<DeferredTask>,
}

impl TaskScheduler for TestTaskScheduler<'_> {
    fn event_tx(&self) -> EventSender {
        self.task_event_tx.clone()
    }

    fn spawn(&mut self, func: TaskFuture) -> Result<TaskHandle, Box<dyn std::error::Error>> {
        let id = self.spawned_tasks.len();

        self.spawned_tasks.push(Some(func));

        let runner_event_tx = self.runner_event_tx.clone();

        Ok(TaskHandle::new(Box::new(move || {
            runner_event_tx.send(TaskRunnerEvent::Dropped(id)).unwrap();
        })))
    }

    fn deferred(&self) -> Box<dyn TaskScheduler> {
        Box::new(DeferredTestTaskScheduler {
            task_event_tx: self.task_event_tx.clone(),
            deferred_tx: self.deferred_tx.clone(),
        })
    }
}

/// Owned scheduler handle into a [`TestTaskRunner`]. Free of borrows, so it can be captured by a
/// `LayoutBuilder` and used to spawn during layout. Spawning sends over a channel the runner drains;
/// cancellation rides a flag the [`TaskHandle`] flips on drop.
pub struct DeferredTestTaskScheduler {
    task_event_tx: EventSender,
    deferred_tx: mpsc::Sender<DeferredTask>,
}

impl TaskScheduler for DeferredTestTaskScheduler {
    fn event_tx(&self) -> EventSender {
        self.task_event_tx.clone()
    }

    fn spawn(&mut self, func: TaskFuture) -> Result<TaskHandle, Box<dyn std::error::Error>> {
        let cancelled = Rc::new(Cell::new(false));

        if self
            .deferred_tx
            .send((func, Rc::clone(&cancelled)))
            .is_err()
        {
            return Err(String::from("the task runner has been dropped").into());
        }

        Ok(TaskHandle::new(Box::new(move || {
            cancelled.set(true);
        })))
    }

    fn deferred(&self) -> Box<dyn TaskScheduler> {
        Box::new(DeferredTestTaskScheduler {
            task_event_tx: self.task_event_tx.clone(),
            deferred_tx: self.deferred_tx.clone(),
        })
    }
}

fn noop_waker() -> Waker {
    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }

    fn noop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);

    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

/// A [`TaskScheduler`] that drops spawned work, for the tests that never poll tasks.
pub struct NoopScheduler {
    event_tx: EventSender,
}

impl NoopScheduler {
    pub fn new() -> Self {
        let (event_tx, _rx) = mpsc::channel();
        Self { event_tx }
    }
}

impl Default for NoopScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskScheduler for NoopScheduler {
    fn event_tx(&self) -> EventSender {
        self.event_tx.clone()
    }

    fn spawn(&mut self, _func: TaskFuture) -> Result<TaskHandle, Box<dyn std::error::Error>> {
        Ok(TaskHandle::new(Box::new(|| {})))
    }

    fn deferred(&self) -> Box<dyn TaskScheduler> {
        Box::new(NoopScheduler::new())
    }
}

/// Builds an [`UpdateCtx`] over a [`NoopScheduler`] and a detached scope, for driving a widget's
/// `create`/`update` (or a rebuild dispatch) directly in a test that does not poll tasks.
pub fn with_ctx<R>(f: impl FnOnce(&mut UpdateCtx) -> R) -> R {
    with_ctx_in(&ProvideScope::new(), f)
}

/// Like [`with_ctx`], but over `provide_scope`, for tests that read a provided value.
pub fn with_ctx_in<R>(provide_scope: &ProvideScope, f: impl FnOnce(&mut UpdateCtx) -> R) -> R {
    let mut scheduler = NoopScheduler::new();
    let mut path = Vec::new();

    f(&mut UpdateCtx::new(
        &mut scheduler,
        &mut path,
        provide_scope,
        &BuildScope::detached(),
    ))
}

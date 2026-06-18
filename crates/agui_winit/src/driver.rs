use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
    task::{Context, Wake, Waker},
    time::Duration,
};

use agui_render::{
    paint::compositing::CompositedFrame,
    pipeline::PipelineOwner,
    prelude::{element::*, render_object::*},
    scheduling::{EventSender, TaskEventMessage, TaskFuture, TaskHandle, TaskScheduler, Vsync},
    view::ViewHandle,
};
use async_executor::LocalExecutor;
use winit::event_loop::EventLoopProxy;

/// Posted through the event-loop proxy to wake the loop when a spawned task becomes ready.
pub(crate) struct WakeUp;

/// Bridges the executor's wakeups to the event loop: a task's waker calls this, and it posts a
/// [`WakeUp`] through the proxy so an otherwise-sleeping loop comes back to tick the ready task.
pub(crate) struct ProxyWaker(pub EventLoopProxy<WakeUp>);

impl Wake for ProxyWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        let _ = self.0.send_event(WakeUp);
    }
}

/// A [`TaskScheduler`] that spawns onto a single-threaded [`LocalExecutor`] and posts task messages
/// back through `event_tx`. Cloning shares the executor, so it carries no borrow and can be captured
/// to spawn later, including during layout.
struct ExecutorScheduler {
    executor: Rc<LocalExecutor<'static>>,
    event_tx: EventSender,
}

impl TaskScheduler for ExecutorScheduler {
    fn event_tx(&self) -> EventSender {
        self.event_tx.clone()
    }

    fn spawn(&mut self, func: TaskFuture) -> Result<TaskHandle, Box<dyn std::error::Error>> {
        // Holding the returned handle keeps the task alive; dropping it drops the `Task`, which cancels.
        let task = self.executor.spawn(func);

        Ok(TaskHandle::new(Box::new(move || drop(task))))
    }

    fn deferred(&self) -> Box<dyn TaskScheduler> {
        Box::new(ExecutorScheduler {
            executor: Rc::clone(&self.executor),
            event_tx: self.event_tx.clone(),
        })
    }
}

/// Drives one window's render subtree from the event loop through a [`PipelineOwner`], which rebuilds
/// what was dirtied and brings layout and paint up to date each frame.
///
/// The widget is wrapped in a [`View`](agui_render::view::View), whose [`ViewHandle`] sizes, hit-tests,
/// and composites the subtree the window presents. Spawned tasks run on a single-threaded executor
/// whose wakeups return to the event loop through a proxy waker.
pub struct WindowDriver {
    executor: Rc<LocalExecutor<'static>>,
    event_tx: EventSender,
    events_rx: mpsc::Receiver<TaskEventMessage>,
    waker: Waker,
    vsync: Vsync,
    owner: PipelineOwner,
    view: ViewHandle,
}

impl WindowDriver {
    /// Mounts `widget` under a fresh pipeline and executor, posting task wakeups through `proxy`.
    pub(crate) fn new<V>(widget: V, vsync: Vsync, proxy: EventLoopProxy<WakeUp>) -> Self
    where
        V: Widget + 'static,
        V::Render: RenderBox,
    {
        let executor = Rc::new(LocalExecutor::new());
        let waker = Waker::from(Arc::new(ProxyWaker(proxy)));
        let (event_tx, events_rx) = mpsc::channel::<TaskEventMessage>();

        let mut scheduler = ExecutorScheduler {
            executor: Rc::clone(&executor),
            event_tx: event_tx.clone(),
        };

        let surface = Rc::new(RefCell::new(None));
        let owner = PipelineOwner::new(
            agui_render::view::View::new(Rc::clone(&surface)).child(widget),
            &mut scheduler,
        );
        let view = surface
            .borrow()
            .clone()
            .expect("a mounted View fills its surface slot");

        Self {
            executor,
            event_tx,
            events_rx,
            waker,
            vsync,
            owner,
            view,
        }
    }

    fn scheduler(&self) -> ExecutorScheduler {
        ExecutorScheduler {
            executor: Rc::clone(&self.executor),
            event_tx: self.event_tx.clone(),
        }
    }

    /// Re-lays and repaints the subtree under new viewport `constraints`.
    pub fn resize(&self, constraints: BoxConstraints) {
        self.view.resize(constraints);
    }

    /// Advances spawned tasks as far as they go and applies the messages they post, settling a chain
    /// of wakeups in one pass rather than one per frame.
    pub fn poll_tasks(&mut self) {
        // Dispatching a message may ready a task, so tasks and messages drain together until neither
        // makes progress. When the executor goes pending the proxy waker is armed, so a later
        // off-thread wakeup re-posts `WakeUp` and brings the loop back.
        let mut cx = Context::from_waker(&self.waker);

        loop {
            let mut ran = false;
            while std::pin::pin!(self.executor.tick())
                .poll(&mut cx)
                .is_ready()
            {
                ran = true;
            }

            let messages = self.events_rx.try_iter().collect::<Vec<_>>();
            let delivered = !messages.is_empty();
            for (path, message) in messages {
                self.owner.dispatch_message(&path, message);
            }

            if !ran && !delivered {
                break;
            }
        }
    }

    /// Whether a frame is owed: the tree was dirtied or an animation is still ticking.
    pub fn needs_frame(&self) -> bool {
        self.owner.is_dirty() || !self.vsync.is_idle()
    }

    /// Brings the subtree up to date for `now` and composites the frame to present.
    pub fn frame(&mut self, now: Duration) -> CompositedFrame {
        // Tasks have already been drained, so apply any rebuild they queued, advance frame callbacks
        // for this frame's time, then lay out and paint what changed.
        let mut scheduler = self.scheduler();
        self.owner.flush_build(&mut scheduler);

        self.vsync.tick(now);

        self.owner.flush_layout();
        self.owner.flush_paint();
        self.view.composite_frame()
    }

    /// The handlers under `position`, most-specific first.
    pub fn hit_test(&self, position: Offset) -> HitTestResult {
        self.view.hit_test(position)
    }
}

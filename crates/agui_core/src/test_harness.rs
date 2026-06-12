use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc,
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

use crate::{
    context::{LayoutCtx, UpdateCtx},
    element::{BuildScope, LeafElement},
    pipeline::{
        PipelineOwner,
        layout::{LayoutPipeline, LayoutScope},
        paint::{PaintPipeline, PaintScope},
    },
    provide::ProvideScope,
    render_object::{
        RenderObject,
        box_layout::{BoxConstraints, RenderBox},
    },
    scheduling::{EventSender, TaskEventMessage, TaskFuture, TaskHandle, TaskScheduler},
    view::{View, ViewHandle},
    widget::Widget,
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

pub struct TestCtx {
    tasks: TestTaskRunner,
    provide: ProvideScope,
    paint: PaintPipeline,
    layout: LayoutPipeline,
}

impl TestCtx {
    pub fn new() -> Self {
        Self {
            tasks: TestTaskRunner::new(),
            provide: ProvideScope::new(),
            paint: PaintPipeline::default(),
            layout: LayoutPipeline::default(),
        }
    }

    /// Seeds `value` as a provided value visible to every reconcile run through this context.
    pub fn with_provided<T: Any>(mut self, value: Rc<T>) -> Self {
        self.provide = self.provide.provide(value);
        self
    }

    /// Runs `f` with a fresh [`UpdateCtx`] over this context. The escape hatch for the reconciles the
    /// typed helpers do not cover: dispatching into an element, or building a shared element directly.
    pub fn run<R>(&mut self, f: impl FnOnce(&mut UpdateCtx) -> R) -> R {
        let mut scheduler = self.tasks.scheduler();
        let mut path = Vec::new();

        f(&mut UpdateCtx::new(
            &mut scheduler,
            &mut path,
            &self.provide,
            &BuildScope::detached(),
            &self.layout,
            &mut self.paint,
            &PaintScope::detached(),
        ))
    }

    /// Builds `widget`'s element and render object.
    pub fn create<W: Widget>(&mut self, widget: W) -> (W::Element, W::Render) {
        self.run(|ctx| widget.create(ctx))
    }

    /// Reconciles `element` and `render` against a new `widget`.
    pub fn update<W: Widget>(
        &mut self,
        widget: W,
        element: &mut W::Element,
        render: &mut W::Render,
    ) {
        self.run(|ctx| widget.update(element, render, ctx));
    }

    /// A [`LayoutCtx`] over this context's pipelines with a detached relayout scope, for laying a
    /// render object out in isolation. Reuse it to lay the same render object out more than once.
    pub fn layout_ctx(&mut self) -> LayoutCtx<'_> {
        LayoutCtx::new(&self.layout, &mut self.paint, LayoutScope::detached())
    }

    /// Builds `widget`'s render object and lays it out once under `constraints`, returning the laid-out
    /// render object. For a render object laid out repeatedly, build it with [`create`](Self::create)
    /// and drive [`layout_ctx`](Self::layout_ctx) directly.
    pub fn laid_out<W>(&mut self, widget: W, constraints: BoxConstraints) -> W::Render
    where
        W: Widget,
        W::Render: RenderBox,
    {
        let (_, mut render) = self.create(widget);
        render.layout(&mut self.layout_ctx(), constraints);

        render
    }

    /// Mounts `widget` as the child of a [`View`] and returns the owner together with the view's handle.
    ///
    /// # Panics
    ///
    /// Panics if the [`View`] does not fill its surface slot at mount, which never happens for a mounted
    /// view.
    pub fn mount_view<V>(&mut self, widget: V) -> (PipelineOwner, ViewHandle)
    where
        V: Widget,
        V::Element: 'static,
        V::Render: RenderObject + RenderBox,
    {
        let surface = Rc::new(RefCell::new(None));

        let owner = {
            let mut scheduler = self.tasks.scheduler();
            PipelineOwner::new(View::new(Rc::clone(&surface)).child(widget), &mut scheduler)
        };

        let view = surface
            .borrow()
            .clone()
            .expect("a mounted View fills its surface slot");

        (owner, view)
    }

    /// A scheduler handle into this context's task runner, for driving an owner's `flush_build`.
    pub fn scheduler(&mut self) -> TestTaskScheduler<'_> {
        self.tasks.scheduler()
    }

    /// Polls every spawned task once. A message a task posts is read back through
    /// [`messages`](Self::messages).
    pub fn poll(&mut self) {
        self.tasks.poll();
    }

    /// Runs every spawned task to completion. Use only for tasks that finish on their own.
    pub fn run_tasks_to_completion(&mut self) {
        self.tasks.run_to_completion();
    }

    /// The messages spawned tasks have posted since the last drain.
    pub fn messages(&self) -> impl Iterator<Item = TaskEventMessage> + '_ {
        self.tasks.messages()
    }
}

impl Default for TestCtx {
    fn default() -> Self {
        Self::new()
    }
}

/// A leaf widget that hands a pre-built render object to the pipeline unchanged, so a test can drive a
/// hand-built render subtree that has no widget of its own. Wrap it in a [`View`] through
/// [`TestCtx::mount_view`] to lay it out, paint it, and hit-test it.
pub struct RawWidget<R> {
    render: R,
}

impl<R> RawWidget<R> {
    pub fn new(render: R) -> Self {
        Self { render }
    }
}

impl<R> Widget for RawWidget<R>
where
    R: RenderObject + RenderBox + 'static,
{
    type Element = LeafElement<R>;

    type Render = R;

    fn create(self, _: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        (LeafElement::new(), self.render)
    }

    fn update(self, _: &mut Self::Element, _: &mut Self::Render, _: &mut UpdateCtx) {}
}

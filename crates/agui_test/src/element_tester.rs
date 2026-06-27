use std::{any::Any, cell::Cell, marker::PhantomData, rc::Rc};

use crate::test_harness::TestCtx;
use agui::{
    pipeline::PipelineOwner,
    prelude::{element::*, render_object::RenderObjectPtr},
};

/// Drives a widget through the build phase against the real [`PipelineOwner`], without laying out or
/// painting it.
///
/// Build one with [`mount`](Self::mount); it reconciles the widget against a new one on
/// [`rebuild`](Self::rebuild) and delivers a message to any element by handle with [`send`](Self::send).
/// Unlike [`WidgetTester`](crate::WidgetTester) it imposes no render type on the widget and never mounts a
/// surface, so it tests an element's lifecycle and routing independently of what, if anything, it renders.
pub struct ElementTester<W: Widget> {
    ctx: TestCtx,
    owner: PipelineOwner,

    /// The render-less root's handle, for reconciling the widget under test against a new one on `rebuild`.
    root: NodeHandle,
    /// The widget-under-test's own handle, for addressing it directly.
    child: NodeHandle,

    _widget: PhantomData<fn() -> W>,
}

impl<W> ElementTester<W>
where
    W: Widget + 'static,
    W::Element: 'static,
{
    /// Mounts `widget` under a render-less root, ready to be reconciled and addressed.
    ///
    /// # Panics
    ///
    /// Panics if the root does not capture its handles at mount, which never happens once it is mounted.
    pub fn mount(widget: W) -> Self {
        let mut ctx = TestCtx::new();

        let root_handle = Rc::new(Cell::new(None));
        let child_handle = Rc::new(Cell::new(None));

        let owner = PipelineOwner::new(
            ElementRoot {
                initial: widget,
                root_handle: Rc::clone(&root_handle),
                child_handle: Rc::clone(&child_handle),
            },
            &mut ctx.scheduler(),
        );

        Self {
            ctx,
            owner,
            root: root_handle
                .get()
                .expect("the root captured its handle at mount"),
            child: child_handle
                .get()
                .expect("the root captured its child's handle at mount"),
            _widget: PhantomData,
        }
    }

    /// Reconciles the widget under test against `widget`, then flushes any rebuilds it triggers.
    pub fn rebuild(&mut self, widget: W) {
        self.owner.dispatch_message(self.root, Box::new(widget));
        self.owner.flush_build(&mut self.ctx.scheduler());
    }

    /// Delivers `message` to the element at `handle`, then flushes any rebuilds it triggers. A message to a
    /// handle whose element is gone is dropped.
    pub fn dispatch(&mut self, handle: NodeHandle, message: Box<dyn Any>) {
        self.owner.dispatch_message(handle, message);
        self.owner.flush_build(&mut self.ctx.scheduler());
    }

    /// Delivers `message` to the element at `handle`, boxing it. See [`dispatch`](Self::dispatch).
    pub fn send<M: 'static>(&mut self, handle: NodeHandle, message: M) {
        self.dispatch(handle, Box::new(message));
    }

    /// Runs every spawned task to completion, then delivers the messages they posted back to their elements,
    /// flushing the rebuilds those messages trigger.
    pub fn run_tasks(&mut self) {
        self.ctx.run_tasks_to_completion();
        let posted = self.ctx.messages().collect::<Vec<_>>();
        for (handle, message) in posted {
            self.dispatch(handle, message);
        }
    }

    /// The handle of the widget under test, for delivering a message straight to it.
    pub fn root_handle(&self) -> NodeHandle {
        self.child
    }

    /// A diagnostics snapshot of the mounted subtree.
    pub fn diagnostics(&self) -> DiagnosticsNode {
        self.owner.diagnostics()
    }
}

/// The render-less reconcilable root [`ElementTester`] mounts the widget under test beneath. It renders
/// nothing of its own and reconciles its child against a new widget delivered as a message, so the tester
/// can re-drive the whole subtree. It records its own handle and its child's at mount.
struct ElementRoot<W> {
    initial: W,
    root_handle: Rc<Cell<Option<NodeHandle>>>,
    child_handle: Rc<Cell<Option<NodeHandle>>>,
}

impl<W> Widget for ElementRoot<W>
where
    W: Widget + 'static,
    W::Element: 'static,
{
    type Element = ElementRootElement<W>;

    type Render = ();

    fn create(self, ctx: &mut CreateCtx) -> ElementRootElement<W> {
        ElementRootElement {
            child: Slot::new(self.initial.create(ctx)),
            pending: None,
            root_handle: self.root_handle,
            child_handle: self.child_handle,
            render: (),
        }
    }

    fn update(self, _ctx: &mut UpdateCtx<'_>, _element: &mut ElementRootElement<W>) {}
}

/// The element of an [`ElementRoot`], holding the mounted child and the widget queued for the next rebuild.
struct ElementRootElement<W: Widget> {
    child: Slot<W::Element>,
    pending: Option<W>,
    root_handle: Rc<Cell<Option<NodeHandle>>>,
    child_handle: Rc<Cell<Option<NodeHandle>>>,
    render: (),
}

// SAFETY: manages its single child only through the cursor child operations, and renders nothing of its own.
unsafe impl<W> Element for ElementRootElement<W>
where
    W: Widget + 'static,
    W::Element: 'static,
{
    type Render = ();

    fn render_object_mut(&mut self) -> &mut () {
        &mut self.render
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<()> {
        RenderObjectPtr::dangling()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.root_handle.set(Some(ctx.handle()));

        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.mount(&mut self.child) };

        self.child_handle.set(Some(self.child.handle()));
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.unmount(&mut self.child) };
    }

    fn rebuild(&mut self, ctx: &mut UpdateCtx<'_>) {
        if let Some(widget) = self.pending.take() {
            // SAFETY: `self.child` is our own slot.
            unsafe { ctx.with_child(&mut self.child, |element, ctx| widget.update(ctx, element)) };
        }
    }

    fn message(&mut self, ctx: &mut MessageCtx<'_>) {
        self.pending = Some(ctx.consume::<W>());
        ctx.request_rebuild();
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.child.get().describe(d)
    }
}

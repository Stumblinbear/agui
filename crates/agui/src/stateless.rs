use agui_core::tree::Slot;

use crate::{
    context::{BuildCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode, DiagnosticsNodeBuilder},
    element::Element,
    pipeline::render_pipeline::SemanticsScope,
    provide::ProvideScope,
    render_object::node::RenderObjectPtr,
    widget::Widget,
};

/// A widget with no state of its own: it builds a subtree from its own fields and the values in scope.
///
/// Implement this for a widget that is pure configuration but still needs to read provided values, which a
/// leaf render-object widget cannot. [`build`](Self::build) runs at mount and on every rebuild: a reconcile
/// with new fields, or a change to a provided value it read.
pub trait StatelessWidget {
    type Child: Widget;

    /// Builds the subtree to show. Read provided values here with
    /// [`BuildCtx::depend_on_provided`](crate::context::BuildCtx::depend_on_provided) so a later change to one
    /// reruns this build.
    fn build(&self, ctx: &mut BuildCtx) -> Self::Child;

    /// Adds this widget's fields to `node`, for a diagnostics dump.
    fn describe<'a>(&self, node: DiagnosticsNodeBuilder<'a>) -> DiagnosticsNodeBuilder<'a> {
        node
    }
}

/// The [`Element`] of a [`StatelessWidget`]. It holds the current widget and the child subtree built from it,
/// and presents the child's render as its own. The child is built at mount, where the element has the handle
/// and scope its dependency reads need.
pub struct StatelessElement<W>
where
    W: StatelessWidget,
{
    widget: W,
    provide_scope: ProvideScope,
    semantics_scope: SemanticsScope,
    child: Option<Slot<<W::Child as Widget>::Element>>,
}

impl<W> StatelessElement<W>
where
    W: StatelessWidget,
{
    /// Wraps `widget`. Its child is built and mounted at mount, not here, so the child's dependency reads see
    /// a real handle and the scope it mounts under.
    pub fn new(widget: W) -> Self {
        Self {
            widget,
            provide_scope: ProvideScope::default(),
            semantics_scope: SemanticsScope::default(),
            child: None,
        }
    }

    fn child(&self) -> &Slot<<W::Child as Widget>::Element> {
        self.child.as_ref().expect("the child is built at mount")
    }

    fn child_mut(&mut self) -> &mut Slot<<W::Child as Widget>::Element> {
        self.child.as_mut().expect("the child is built at mount")
    }
}

impl<W> StatelessElement<W>
where
    W: StatelessWidget + 'static,
{
    /// Reconciles this element against a new `widget`: swaps in its fields, rebuilds, and reconciles the child
    /// in place. A stateless widget's [`Widget::update`] forwards here.
    pub fn update_widget(&mut self, ctx: &mut UpdateCtx<'_>, widget: W) {
        self.widget = widget;
        self.rebuild_child(ctx);
    }

    fn rebuild_child(&mut self, ctx: &mut UpdateCtx<'_>) {
        let scope = self.provide_scope;
        let semantics = self.semantics_scope;

        ctx.with_provide_scope(scope, |ctx| {
            ctx.with_semantics_scope(semantics, |ctx| {
                let child = ctx.build(|ctx| self.widget.build(ctx));

                // SAFETY: `self.child` is our own slot, built at mount.
                unsafe {
                    ctx.with_child(self.child_mut(), |element, ctx| child.update(ctx, element));
                }
            });
        });
    }
}

// SAFETY: builds and reconciles its single child only through the cursor child operations and forwards render
// resolution to it.
unsafe impl<W> Element for StatelessElement<W>
where
    W: StatelessWidget + 'static,
{
    type Render = <W::Child as Widget>::Render;

    fn render_object_ptr(&self) -> RenderObjectPtr<Self::Render> {
        self.child().get().render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.provide_scope = ctx.provide_scope();
        self.semantics_scope = ctx.semantics_scope();

        let scope = self.provide_scope;
        let semantics = self.semantics_scope;

        ctx.with_provide_scope(scope, |ctx| {
            ctx.with_semantics_scope(semantics, |ctx| {
                let child = ctx.build(|ctx| self.widget.build(ctx));
                let element = ctx.inflate(|ctx| child.create(ctx));

                self.child = Some(Slot::new(element));

                // SAFETY: `self.child` is our own slot, just built.
                unsafe { ctx.mount(self.child.as_mut().expect("just built")) };
            });
        });
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.unmount(self.child_mut()) };
    }

    fn rebuild(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.rebuild_child(ctx);
    }

    fn dependency_changed(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.rebuild_child(ctx);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.widget
            .describe(d.node_for::<W>())
            .child(|d| self.child().get().describe(d))
            .finish()
    }
}

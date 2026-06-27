use agui_core::tree::Slot;

use crate::{
    context::{BuildCtx, MessageCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode, DiagnosticsNodeBuilder},
    element::Element,
    pipeline::render_pipeline::SemanticsScope,
    provide::ProvideScope,
    render_object::node::RenderObjectPtr,
    widget::Widget,
};

/// State that persists across rebuilds and produces the subtree to show for it.
///
/// Implement this on the data a widget owns; [`build`](Self::build) reads that data and returns the child to
/// display.
pub trait WidgetState {
    type Widget: Widget;

    type Child: Widget;

    fn init_state(ctx: &mut BuildCtx, widget: Self::Widget) -> Self;

    fn did_update_widget(&mut self, ctx: &mut BuildCtx, widget: Self::Widget);

    /// Reacts to a change in a value this state depends on, before the rebuild's [`build`](Self::build).
    /// Recompute derived state or re-establish anything keyed by the dependency here. The default does
    /// nothing.
    fn did_change_dependencies(&mut self, ctx: &mut BuildCtx) {
        let _ = ctx;
    }

    /// Builds the subtree to show for the current state.
    fn build(&self, ctx: &mut BuildCtx) -> Self::Child;

    /// Adds this state's data to `node`, for a diagnostics dump.
    fn describe<'a>(&self, node: DiagnosticsNodeBuilder<'a>) -> DiagnosticsNodeBuilder<'a> {
        node
    }
}

/// A mutation applied to a state to change it, delivered as a message to a stateful widget.
///
/// Deliver one to the widget's element to mutate its state and schedule a rebuild of its subtree.
pub type SetState<S> = Box<dyn FnOnce(&mut S)>;

/// The [`Element`] of a stateful widget. It owns the state and the child subtree built from it, and presents
/// the child's render as its own. The child is built at mount, where the element has the handle and scope its
/// dependency reads need.
pub struct StatefulElement<S>
where
    S: WidgetState,
{
    state: S,
    provide_scope: ProvideScope,
    semantics: SemanticsScope,
    child: Option<Slot<<S::Child as Widget>::Element>>,
}

impl<S> StatefulElement<S>
where
    S: WidgetState,
{
    /// Wraps `state`. Its child is built and mounted at mount, not here, so the child's dependency reads see
    /// a real handle and the scope it mounts under.
    pub fn new(state: S) -> Self {
        Self {
            state,
            provide_scope: ProvideScope::default(),
            semantics: SemanticsScope::default(),
            child: None,
        }
    }

    fn child(&self) -> &Slot<<S::Child as Widget>::Element> {
        self.child.as_ref().expect("the child is built at mount")
    }

    fn child_mut(&mut self) -> &mut Slot<<S::Child as Widget>::Element> {
        self.child.as_mut().expect("the child is built at mount")
    }
}

impl<S> StatefulElement<S>
where
    S: WidgetState + 'static,
{
    /// Reconciles this element against new `widget` props: applies [`WidgetState::did_update_widget`],
    /// rebuilds, and reconciles the child in place. A stateful widget's [`Widget::update`] forwards here.
    pub fn update_widget(&mut self, ctx: &mut UpdateCtx<'_>, widget: S::Widget) {
        let scope = self.provide_scope;
        let semantics = self.semantics;

        ctx.with_provide_scope(scope, |ctx| {
            ctx.with_semantics_scope(semantics, |ctx| {
                ctx.build(|ctx| self.state.did_update_widget(ctx, widget));

                let child = ctx.build(|ctx| self.state.build(ctx));

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
unsafe impl<S> Element for StatefulElement<S>
where
    S: WidgetState + 'static,
{
    type Render = <S::Child as Widget>::Render;

    fn render_object_ptr(&self) -> RenderObjectPtr<Self::Render> {
        self.child().get().render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.provide_scope = ctx.provide_scope();
        self.semantics = ctx.semantics_scope();

        let scope = self.provide_scope;
        let semantics = self.semantics;

        ctx.with_provide_scope(scope, |ctx| {
            ctx.with_semantics_scope(semantics, |ctx| {
                let child = ctx.build(|ctx| self.state.build(ctx));
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
        let scope = self.provide_scope;
        let semantics = self.semantics;

        ctx.with_provide_scope(scope, |ctx| {
            ctx.with_semantics_scope(semantics, |ctx| {
                let child = ctx.build(|ctx| self.state.build(ctx));
                // SAFETY: `self.child` is our own slot.
                unsafe {
                    ctx.with_child(self.child_mut(), |element, ctx| child.update(ctx, element));
                }
            });
        });
    }

    fn dependency_changed(&mut self, ctx: &mut UpdateCtx<'_>) {
        let scope = self.provide_scope;
        let semantics = self.semantics;

        ctx.with_provide_scope(scope, |ctx| {
            ctx.with_semantics_scope(semantics, |ctx| {
                ctx.build(|ctx| self.state.did_change_dependencies(ctx));
                let child = ctx.build(|ctx| self.state.build(ctx));
                // SAFETY: `self.child` is our own slot.
                unsafe {
                    ctx.with_child(self.child_mut(), |element, ctx| child.update(ctx, element));
                }
            });
        });
    }

    fn message(&mut self, ctx: &mut MessageCtx<'_>) {
        let apply: SetState<S> = ctx.consume();
        apply(&mut self.state);
        ctx.request_rebuild();
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.state
            .describe(d.node_for::<S::Widget>())
            .child(|d| self.child().get().describe(d))
            .finish()
    }
}

use agui_core::tree::Slot;

use crate::{
    context::{BuildCtx, MessageCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode, DiagnosticsNodeBuilder},
    element::Element,
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
    scope: ProvideScope,
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
            scope: ProvideScope::default(),
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
        let scope = self.scope;
        ctx.with_scope(scope, |ctx| {
            ctx.build(|ctx| self.state.did_update_widget(ctx, widget));
            let child = ctx.build(|ctx| self.state.build(ctx));
            // SAFETY: `self.child` is our own slot, built at mount.
            unsafe {
                ctx.with_child(self.child_mut(), |element, ctx| child.update(ctx, element));
            }
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

    fn render_object_mut(&mut self) -> &mut Self::Render {
        self.child_mut().get_mut().render_object_mut()
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<Self::Render> {
        self.child().get().render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.scope = ctx.provide();

        let scope = self.scope;
        ctx.with_scope(scope, |ctx| {
            let child = ctx.build(|ctx| self.state.build(ctx));
            let element = ctx.inflate(|ctx| child.create(ctx));
            self.child = Some(Slot::new(element));
            // SAFETY: `self.child` is our own slot, just built.
            unsafe { ctx.mount(self.child.as_mut().expect("just built")) };
        });
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.unmount(self.child_mut()) };
    }

    fn rebuild(&mut self, ctx: &mut UpdateCtx<'_>) {
        let scope = self.scope;
        ctx.with_scope(scope, |ctx| {
            let child = ctx.build(|ctx| self.state.build(ctx));
            // SAFETY: `self.child` is our own slot.
            unsafe {
                ctx.with_child(self.child_mut(), |element, ctx| child.update(ctx, element));
            }
        });
    }

    fn dependency_changed(&mut self, ctx: &mut UpdateCtx<'_>) {
        let scope = self.scope;
        ctx.with_scope(scope, |ctx| {
            ctx.build(|ctx| self.state.did_change_dependencies(ctx));
            let child = ctx.build(|ctx| self.state.build(ctx));
            // SAFETY: `self.child` is our own slot.
            unsafe {
                ctx.with_child(self.child_mut(), |element, ctx| child.update(ctx, element));
            }
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::{SetState, StatefulElement, WidgetState};
    use crate::context::{BuildCtx, CreateCtx, UpdateCtx};
    use crate::provide::Provide;
    use crate::test_fixtures::Leaf;
    use crate::test_harness::WidgetTester;
    use crate::widget::Widget;

    // State holding a count, building a child that writes the count to a shared cell when it reconciles, so a
    // test sees which count the rebuilt child carried.
    struct Counter {
        count: u32,
        observed: Rc<Cell<u32>>,
    }

    impl WidgetState for Counter {
        type Widget = CounterWidget;
        type Child = Leaf;

        fn init_state(_: &mut BuildCtx, widget: Self::Widget) -> Self {
            Counter {
                count: widget.count,
                observed: widget.observed,
            }
        }

        fn did_update_widget(&mut self, _: &mut BuildCtx, widget: Self::Widget) {
            self.count = widget.count;
            self.observed = widget.observed;
        }

        fn build(&self, _: &mut BuildCtx) -> Self::Child {
            let count = self.count;
            let observed = Rc::clone(&self.observed);
            Leaf::new().on_update(move |_| observed.set(count))
        }
    }

    struct CounterWidget {
        count: u32,
        observed: Rc<Cell<u32>>,
    }

    impl Widget for CounterWidget {
        type Element = StatefulElement<Counter>;

        type Render = ();

        fn create(self, ctx: &mut CreateCtx) -> Self::Element {
            let state = ctx.build(|ctx| Counter::init_state(ctx, self));
            StatefulElement::new(state)
        }

        fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
            let scope = element.scope;
            ctx.with_scope(scope, |ctx| {
                ctx.build(|ctx| element.state.did_update_widget(ctx, self));
                let child = ctx.build(|ctx| element.state.build(ctx));
                // SAFETY: `element.child` is the stateful element's own slot.
                unsafe {
                    ctx.with_child(element.child_mut(), |c, ctx| child.update(ctx, c));
                }
            });
        }
    }

    #[test]
    fn set_state_rebuilds_the_child() {
        let observed = Rc::new(Cell::new(0));
        let mut tester = WidgetTester::mount(CounterWidget {
            count: 0,
            observed: Rc::clone(&observed),
        });
        let handle = tester.root_handle();

        let bump: SetState<Counter> = Box::new(|state| state.count = 7);
        tester.dispatch(handle, Box::new(bump));
        assert_eq!(observed.get(), 7);

        let bump: SetState<Counter> = Box::new(|state| state.count = 12);
        tester.dispatch(handle, Box::new(bump));
        assert_eq!(observed.get(), 12);
    }

    // A stateful consumer of a provided `usize`. It records the value its build saw and counts its
    // dependency-change hook, so a test can confirm a `Provide` change runs `did_change_dependencies` and
    // rebuilds the consumer with the new value.
    struct Consumer {
        seen: Rc<Cell<usize>>,
        dependency_changes: Rc<Cell<u32>>,
    }

    impl WidgetState for Consumer {
        type Widget = ConsumerWidget;
        type Child = Leaf;

        fn init_state(_: &mut BuildCtx, widget: Self::Widget) -> Self {
            Consumer {
                seen: widget.seen,
                dependency_changes: widget.dependency_changes,
            }
        }

        fn did_update_widget(&mut self, _: &mut BuildCtx, widget: Self::Widget) {
            self.seen = widget.seen;
            self.dependency_changes = widget.dependency_changes;
        }

        fn did_change_dependencies(&mut self, _: &mut BuildCtx) {
            self.dependency_changes
                .set(self.dependency_changes.get() + 1);
        }

        fn build(&self, ctx: &mut BuildCtx) -> Self::Child {
            let value = ctx
                .depend_on_provided::<usize>()
                .as_deref()
                .copied()
                .unwrap_or(0);
            self.seen.set(value);
            Leaf::new()
        }
    }

    struct ConsumerWidget {
        seen: Rc<Cell<usize>>,
        dependency_changes: Rc<Cell<u32>>,
    }

    impl Widget for ConsumerWidget {
        type Element = StatefulElement<Consumer>;

        type Render = ();

        fn create(self, ctx: &mut CreateCtx) -> Self::Element {
            let state = ctx.build(|ctx| Consumer::init_state(ctx, self));
            StatefulElement::new(state)
        }

        fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
            let scope = element.scope;
            ctx.with_scope(scope, |ctx| {
                ctx.build(|ctx| element.state.did_update_widget(ctx, self));
                let child = ctx.build(|ctx| element.state.build(ctx));
                // SAFETY: `element.child` is the stateful element's own slot.
                unsafe {
                    ctx.with_child(element.child_mut(), |c, ctx| child.update(ctx, c));
                }
            });
        }
    }

    #[test]
    fn a_dependency_change_rebuilds_a_stateful_consumer_with_the_new_value() {
        let seen = Rc::new(Cell::new(0));
        let dependency_changes = Rc::new(Cell::new(0));

        let mut tester = WidgetTester::mount(Provide::new(1usize).child(ConsumerWidget {
            seen: Rc::clone(&seen),
            dependency_changes: Rc::clone(&dependency_changes),
        }));
        assert_eq!(
            seen.get(),
            1,
            "the consumer read the value when it built at mount"
        );
        assert_eq!(dependency_changes.get(), 0, "no dependency change at mount");

        tester.rebuild(Provide::new(2usize).child(ConsumerWidget {
            seen: Rc::clone(&seen),
            dependency_changes: Rc::clone(&dependency_changes),
        }));
        assert_eq!(seen.get(), 2, "the consumer rebuilt with the new value");
        assert_eq!(
            dependency_changes.get(),
            1,
            "its dependency-change hook ran once"
        );
    }
}

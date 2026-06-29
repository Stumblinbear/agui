use std::{any::Any, marker::PhantomData};

use bon::Builder;

use agui_core::{provide::ProvideNode, tree::Slot};

pub use agui_core::provide::ProvideScope;

use crate::{
    context::{CreateCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::Element,
    render_object::node::RenderObjectPtr,
    widget::Widget,
};

/// A widget that makes one value available to its subtree.
///
/// # Examples
///
/// ```ignore
/// Provide::new(theme).child(page)
/// ```
#[derive(Builder)]
#[builder(start_fn = new)]
#[builder(finish_fn = child)]
pub struct Provide<V, Child> {
    #[builder(start_fn)]
    value: V,

    #[builder(finish_fn)]
    child: Child,
}

impl<V, Child> Widget for Provide<V, Child>
where
    V: PartialEq + 'static,
    Child: Widget,
{
    type Element = ProvideElement<V, Child::Element>;

    type Render = Child::Render;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let child_element = self.child.create(ctx);

        ProvideElement {
            node: Box::new(ProvideNode::new(self.value)),
            child: Slot::new(child_element),
            _value: PhantomData,
        }
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        // The value changes through the node's interior mutability, never a `&mut` to the node itself, so a
        // descendant's scope pointer into it stays valid across this reconcile.
        let changed = element
            .node
            .current::<V>()
            .is_none_or(|current| *current != self.value);

        if changed {
            for dependent in element.node.replace(self.value) {
                ctx.mark_dependency_changed(dependent);
            }
        }

        let scope = element.node.scope();

        // SAFETY: `element.child` is the element's own slot. Provide is transparent: it only extends the
        // scope, reconciling the child in place.
        ctx.with_provide_scope(scope, |ctx| unsafe {
            ctx.with_child(&mut element.child, |child, ctx| {
                self.child.update(ctx, child);
            });
        });
    }
}

/// The [`Element`] of a [`Provide`]. It holds the value inline and the child it wraps, and exposes the value
/// to its subtree by extending the scope when it mounts.
pub struct ProvideElement<V, C> {
    /// Boxed into its own allocation so a descendant's scope pointer into it survives this element being
    /// reconciled through `&mut`, which retags only the element's own allocation, not the node's.
    node: Box<ProvideNode>,
    child: Slot<C>,
    _value: PhantomData<fn() -> V>,
}

// SAFETY: reconciles its single child only through the cursor child operations and forwards render resolution
// to it.
unsafe impl<V, C> Element for ProvideElement<V, C>
where
    V: Any,
    C: Element,
{
    type Render = C::Render;

    fn render_object_ptr(&self) -> RenderObjectPtr<Self::Render> {
        self.child.get().render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.node.set_parent(ctx.provide_scope());

        let scope = self.node.scope();

        // SAFETY: `self.child` is our own slot, and `self.node` is pinned now that this element is mounted,
        // so the scope it hands down stays valid for the whole subtree's life.
        ctx.with_provide_scope(scope, |ctx| unsafe { ctx.mount(&mut self.child) });
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.unmount(&mut self.child) };
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node(format!("Provide<{}>", Diagnostics::short_type_name::<V>()))
            .child(|d| self.child.get().describe(d))
            .finish()
    }
}

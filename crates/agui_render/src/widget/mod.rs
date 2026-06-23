use std::any::TypeId;

use crate::{
    context::{CreateCtx, UpdateCtx},
    element::Element,
    key::AnyKeyable,
};

mod any_widget;
mod list;

pub use any_widget::*;
pub use list::*;

/// The immutable description of a piece of the tree. A `Widget` is consumed to build its persistent
/// [`Element`], which holds state and children, and consumed again on each reconcile to sync it.
///
/// The type of its render object is [`Render`](Self::Render).
pub trait Widget {
    type Element: Element<Render = Self::Render>;

    type Render: ?Sized;

    /// Builds this widget's persistent [`Element`], which owns the render object this widget produces,
    /// consuming the description. Called once, when the widget first enters the tree; child elements are
    /// built recursively through `ctx`. The element-tree node is registered at mount, where the element also
    /// wires its render object's child edges to its now-pinned children's render objects.
    fn create(self, ctx: &mut CreateCtx) -> Self::Element;

    /// Reconciles `element` against this new description, consuming it. Called when the parent supplies a new
    /// widget of the same type; the widget updates its render object through `element` and reconciles the
    /// element's own children via `ctx`.
    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element);

    /// The type identity used, together with [`key`](Self::key), to decide whether a new widget reconciles
    /// an existing element in place rather than replacing it. Type-erased widgets report their concrete
    /// inner widget's identity.
    fn widget_type_id(&self) -> TypeId
    where
        Self: Sized + 'static,
    {
        TypeId::of::<Self>()
    }

    /// This is an implementation detail of element keys and should not be overriden by any user code.
    fn key(&self) -> Option<&dyn AnyKeyable> {
        None
    }
}

impl Widget for () {
    type Element = ();

    type Render = ();

    fn create(self, _ctx: &mut CreateCtx) {}

    fn update(self, _ctx: &mut UpdateCtx<'_>, (): &mut ()) {}
}

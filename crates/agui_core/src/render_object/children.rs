use crate::render_object::node::RenderNode;

/// A render object that owns a single child render object.
///
/// The render of every single-child widget implements this so its element can reach the child's
/// render to reconcile it in place.
pub trait SingleChildRenderObject {
    /// The child's render object type.
    type Child;

    /// Runs `f` on the child render object, for the element to reconcile against a new child widget.
    fn with_child<R>(&mut self, f: impl FnOnce(&mut Self::Child) -> R) -> R;
}

/// A render object that owns an ordered list of child render objects.
pub trait MultiChildRenderObject {
    /// The child's render object type.
    type Child;

    /// Removes the current children, yielding them in order.
    fn take_children(&mut self) -> Vec<RenderNode<Self::Child>>;

    /// Installs `children` as the new ordered child list.
    fn set_children(&mut self, children: Vec<RenderNode<Self::Child>>);

    /// Runs `f` on the child render object at `index`, for the element to reconcile against a new child widget.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    fn with_child<R>(&mut self, index: usize, f: impl FnOnce(&mut Self::Child) -> R) -> R;
}

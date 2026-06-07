use crate::render_object::node::RenderNode;

/// A render object that owns an ordered list of child render objects.
pub trait MultiChildRender {
    /// The child's render object type.
    type Child;

    /// Removes the current children, yielding them in order.
    fn take_children(&mut self) -> Vec<RenderNode<Self::Child>>;

    /// Installs `children` as the new ordered child list.
    fn set_children(&mut self, children: Vec<RenderNode<Self::Child>>);
}

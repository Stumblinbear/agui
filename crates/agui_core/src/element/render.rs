use crate::{element::RenderObjectCreateContext, render::object::RenderObject, widget::Widget};

use super::{lifecycle::ElementLifecycle, RenderObjectUpdateContext};

pub trait ElementRender: ElementLifecycle {
    fn children(&self) -> Vec<Widget>;

    /// Creates a new render object for the current element.
    ///
    /// This must always create a new render object, and should never contain any data derived
    /// from, or containing references to, any previously created render object.
    fn create_render_object(&self, ctx: &mut RenderObjectCreateContext) -> RenderObject;

    /// Returns true if the given render object is valid for the current element and can be
    /// used as-is.
    ///
    /// Returns false if the given render object is not valid for the current element and a
    /// new one must be created. This does not prevent the render object from being used in
    /// another element.
    fn is_valid_render_object(&self, render_object: &RenderObject) -> bool;

    /// This is called when the element has been rebuilt with a new widget and the render
    /// object remained valid. The render object should be updated to reflect the parameters
    /// of the new widget.
    ///
    /// It's entirely possible that the given render object was never owned by this element
    /// at any point and was instead created by another element at the same position in the
    /// tree, so all necessary parameters *must* be updated.
    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut RenderObject,
    );
}

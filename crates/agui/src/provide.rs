use bon::Builder;

pub use agui_core::provide::{ProvideElement, ProvideScope};

use crate::{
    context::{CreateCtx, UpdateCtx},
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
        ProvideElement::new(self.value, self.child.create(ctx))
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        // SAFETY: the closure reconciles only the element's own child slot, `update`'s precondition.
        unsafe {
            element.update(ctx, self.value, |child, ctx| self.child.update(ctx, child));
        }
    }
}

use std::{marker::PhantomData, ops::Deref};

use crate::{
    element::{context::ElementIntrinsicSizeContext, Element, ElementId},
    unit::{Data, IntrinsicDimension},
    util::tree::Tree,
    widget::{WidgetState, WidgetView},
};

use super::{ContextWidget, ContextWidgetLayout, ContextWidgetState};

pub struct IntrinsicSizeContext<'ctx, W>
where
    W: WidgetView,
{
    pub(crate) phantom: PhantomData<W>,

    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,
    pub(crate) state: &'ctx dyn Data,

    pub(crate) children: &'ctx [ElementId],
}

impl<W> ContextWidget for IntrinsicSizeContext<'_, W>
where
    W: WidgetView,
{
    type Widget = W;

    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<W> ContextWidgetState for IntrinsicSizeContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    type Widget = W;

    fn get_state(&self) -> &W::State {
        self.state.downcast_ref().unwrap()
    }
}

impl<'ctx, W> ContextWidgetLayout<'ctx> for IntrinsicSizeContext<'ctx, W>
where
    W: WidgetView,
{
    fn get_children(&self) -> &'ctx [ElementId] {
        self.children
    }

    fn compute_intrinsic_size(
        &mut self,
        child_id: ElementId,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        self.element_tree
            .with(child_id, |element_tree, element| {
                element.intrinsic_size(
                    ElementIntrinsicSizeContext {
                        element_tree,

                        element_id: child_id,
                    },
                    dimension,
                    cross_extent,
                )
            })
            .expect("child element missing during layout")
    }
}

impl<W> Deref for IntrinsicSizeContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    type Target = W::State;

    fn deref(&self) -> &Self::Target {
        self.state.downcast_ref().unwrap()
    }
}

use std::{marker::PhantomData, ops::Deref};

use crate::{
    element::{
        context::{ElementIntrinsicSizeContext, ElementLayoutContext},
        Element, ElementId,
    },
    unit::{Constraints, Data, IntrinsicDimension, Offset, Size},
    util::tree::Tree,
    widget::{WidgetState, WidgetView},
};

use super::{ContextWidget, ContextWidgetLayout, ContextWidgetState};

pub struct LayoutContext<'ctx, W>
where
    W: WidgetView,
{
    pub(crate) phantom: PhantomData<W>,

    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,
    pub(crate) state: &'ctx dyn Data,

    pub(crate) children: &'ctx [ElementId],
    pub(crate) offsets: &'ctx mut [Option<Offset>],
}

impl<W> ContextWidget for LayoutContext<'_, W>
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

impl<W> ContextWidgetState for LayoutContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    type Widget = W;

    fn get_state(&self) -> &W::State {
        self.state.downcast_ref().unwrap()
    }
}

impl<'ctx, W> ContextWidgetLayout<'ctx> for LayoutContext<'ctx, W>
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
impl<'ctx, W> LayoutContext<'ctx, W>
where
    W: WidgetView,
{
    pub fn compute_layout(
        &mut self,
        child_id: ElementId,
        constraints: impl Into<Constraints>,
    ) -> Size {
        let constraints = constraints.into();

        self.element_tree
            .with(child_id, |element_tree, element| {
                element.layout(
                    ElementLayoutContext {
                        element_tree,

                        element_id: child_id,
                    },
                    constraints,
                )
            })
            .expect("child element missing during layout")
    }

    pub fn set_offsets(&mut self, offsets: &[Offset]) {
        assert_eq!(
            self.offsets.len(),
            offsets.len(),
            "when using set_offsets, the length must match the number of children"
        );

        self.offsets
            .iter_mut()
            .zip(offsets.iter())
            .for_each(|(pos, new_pos)| *pos = Some(*new_pos));
    }

    pub fn set_offset(&mut self, index: usize, offset: impl Into<Offset>) {
        self.offsets[index] = Some(offset.into());
    }
}

impl<W> Deref for LayoutContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    type Target = W::State;

    fn deref(&self) -> &Self::Target {
        self.state.downcast_ref().unwrap()
    }
}

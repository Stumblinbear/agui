use crate::{
    callback::{Callback, CallbackContext},
    element::{Element, ElementId},
    unit::{Data, IntrinsicDimension},
    util::tree::Tree,
    widget::WidgetView,
};

mod build;
mod intrinsic_size;
mod layout;
mod paint;

pub use build::*;
pub use intrinsic_size::*;
pub use layout::*;
pub use paint::*;

use super::{InheritedWidget, WidgetState};

pub trait ContextWidget {
    type Widget: WidgetView;

    fn get_elements(&self) -> &Tree<ElementId, Element>;

    fn get_element_id(&self) -> ElementId;
}

pub trait ContextWidgetMut: ContextWidget {
    fn depend_on_inherited_widget<I>(&mut self) -> Option<&mut I>
    where
        I: InheritedWidget + 'static;

    fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: Data,
        F: Fn(&mut CallbackContext<Self::Widget>, &A) + 'static;
}

pub trait ContextWidgetState {
    type Widget: WidgetState;

    fn get_state(&self) -> &<Self::Widget as WidgetState>::State;
}

pub trait ContextWidgetStateMut: ContextWidgetState {
    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut <Self::Widget as WidgetState>::State);
}

pub trait ContextWidgetLayout<'ctx>: ContextWidget {
    fn get_children(&self) -> &'ctx [ElementId];

    fn get_child(&self) -> Option<ElementId> {
        let children = self.get_children();

        assert_eq!(
            children.len(),
            1,
            "get_child may only be called on widgets with a single child"
        );

        children.first().copied()
    }

    fn compute_intrinsic_size(
        &mut self,
        child_id: ElementId,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32;
}

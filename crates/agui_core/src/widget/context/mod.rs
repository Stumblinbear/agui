use crate::{
    callback::{Callback, CallbackContext},
    element::{Element, ElementId},
    unit::Data,
    util::tree::Tree,
    widget::Widget,
};

mod build;
mod layout;
mod paint;

pub use build::*;
pub use layout::*;
pub use paint::*;

use super::{InheritedWidget, WidgetState};

pub trait ContextWidget {
    type Widget: Widget;

    fn get_elements(&self) -> &Tree<ElementId, Element>;

    fn get_element_id(&self) -> ElementId;

    fn get_widget(&self) -> &Self::Widget;
}

pub trait ContextStatefulWidget: ContextWidget
where
    Self::Widget: WidgetState,
{
    fn get_state(&self) -> &<Self::Widget as WidgetState>::State;

    fn get_state_mut(&mut self) -> &mut <Self::Widget as WidgetState>::State;

    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut <Self::Widget as WidgetState>::State);
}

pub trait ContextWidgetMut: ContextWidget {
    fn depend_on_inherited_widget<I>(&mut self) -> Option<&mut I::State>
    where
        I: InheritedWidget + 'static;

    fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: Data,
        F: Fn(&mut CallbackContext<Self::Widget>, &A) + 'static;
}

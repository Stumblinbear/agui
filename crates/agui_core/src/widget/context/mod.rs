use crate::{
    callback::{Callback, CallbackContext},
    element::{Element, ElementId},
    unit::Data,
    util::tree::Tree,
    widget::WidgetView,
};

mod build;
mod layout;
mod paint;

pub use build::*;
pub use layout::*;
pub use paint::*;

use super::{InheritedWidget, WidgetState};

pub trait ContextWidget {
    type Widget: WidgetView;

    fn get_elements(&self) -> &Tree<ElementId, Element>;

    fn get_element_id(&self) -> ElementId;
}

pub trait ContextWidgetMut: ContextWidget {
    fn depend_on_inherited_widget<I>(&mut self) -> Option<&mut <I as WidgetState>::State>
    where
        I: InheritedWidget + 'static;

    fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: Data,
        F: Fn(&mut CallbackContext<Self::Widget>, &A) + 'static;
}

pub trait ContextStatefulWidget {
    type Widget: WidgetState;

    fn get_state(&self) -> &<Self::Widget as WidgetState>::State;

    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut <Self::Widget as WidgetState>::State);
}

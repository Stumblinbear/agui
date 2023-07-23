use crate::{
    callback::{Callback, CallbackContext},
    element::{Element, ElementId},
    unit::Data,
    util::tree::Tree,
};

use super::InheritedWidget;

pub trait ContextWidget<W> {
    fn get_elements(&self) -> &Tree<ElementId, Element>;

    fn get_element_id(&self) -> ElementId;
}

pub trait ContextWidgetMut<W>: ContextWidget<W> {
    fn depend_on_inherited_widget<I>(&mut self) -> Option<&mut I>
    where
        I: InheritedWidget + 'static;

    fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: Data,
        F: Fn(&mut CallbackContext<W>, &A) + 'static;
}

use std::rc::Rc;

use agui_core::{
    plugin::context::ContextPluginsMut,
    widget::{AnyWidget, ContextWidget},
};

use crate::{
    element::{InheritedElement, InheritedWidget},
    plugin::InheritancePlugin,
};

pub trait ContextInheritedMut {
    fn depend_on_inherited_widget<I>(&mut self) -> Option<Rc<I>>
    where
        I: AnyWidget + InheritedWidget;
}

impl<'ctx, C> ContextInheritedMut for C
where
    C: ContextWidget + ContextPluginsMut<'ctx>,
{
    fn depend_on_inherited_widget<I>(&mut self) -> Option<Rc<I>>
    where
        I: AnyWidget + InheritedWidget,
    {
        let element_id = self.get_element_id();

        let inheritance_plugin = self.get_plugins_mut().get_mut::<InheritancePlugin>()?;

        if let Some(element_id) = inheritance_plugin.depend_on_inherited_element::<I>(element_id) {
            let inherited_element = self
                .get_elements()
                .get(element_id)
                .expect("found an inherited widget but it does not exist exist in the tree")
                .downcast::<InheritedElement<I>>()
                .expect("inherited element downcast failed");

            Some(inherited_element.get_inherited_widget())
        } else {
            None
        }
    }
}

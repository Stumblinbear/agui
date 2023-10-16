use std::rc::Rc;

use agui_core::plugin::context::ContextPlugins;
use agui_core::{element::ContextElement, plugin::context::ContextPluginsMut, widget::AnyWidget};

use crate::{
    element::{InheritedElement, InheritedWidget},
    plugin::InheritancePlugin,
};

pub trait ContextInherited {
    fn find_inherited_widget<I>(&self) -> Option<Rc<I>>
    where
        I: AnyWidget + InheritedWidget;
}

pub trait ContextInheritedMut {
    fn depend_on_inherited_widget<I>(&mut self) -> Option<Rc<I>>
    where
        I: AnyWidget + InheritedWidget;
}

impl<'ctx, C> ContextInherited for C
where
    C: ContextElement + ContextPlugins<'ctx>,
{
    fn find_inherited_widget<I>(&self) -> Option<Rc<I>>
    where
        I: AnyWidget + InheritedWidget,
    {
        let element_id = self.get_element_id();

        let Some(inheritance_plugin) = self.get_plugins().get::<InheritancePlugin>() else {
            tracing::warn!("InheritancePlugin not found in the context");

            return None;
        };

        if let Some(element_id) = inheritance_plugin.find_inherited_element::<I>(element_id) {
            let inherited_element = self
                .get_elements()
                .get(element_id)
                .expect("found an inherited widget but it does not exist exist in the tree")
                .downcast::<InheritedElement<I>>()
                .expect("inherited element downcast failed");

            Some(Rc::clone(&inherited_element.widget))
        } else {
            None
        }
    }
}

impl<'ctx, C> ContextInheritedMut for C
where
    C: ContextElement + ContextPluginsMut<'ctx>,
{
    fn depend_on_inherited_widget<I>(&mut self) -> Option<Rc<I>>
    where
        I: AnyWidget + InheritedWidget,
    {
        let element_id = self.get_element_id();

        let Some(inheritance_plugin) = self.get_plugins_mut().get_mut::<InheritancePlugin>() else {
            tracing::warn!("InheritancePlugin not found in the context");

            return None;
        };

        if let Some(element_id) = inheritance_plugin.depend_on_inherited_element::<I>(element_id) {
            let inherited_element = self
                .get_elements()
                .get(element_id)
                .expect("found an inherited widget but it does not exist exist in the tree")
                .downcast::<InheritedElement<I>>()
                .expect("inherited element downcast failed");

            Some(Rc::clone(&inherited_element.widget))
        } else {
            None
        }
    }
}

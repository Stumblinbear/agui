use std::rc::Rc;

use agui_core::element::ContextElements;
use agui_core::plugin::context::ContextPlugins;
use agui_core::{element::ContextElement, plugin::context::ContextPluginsMut, widget::AnyWidget};

use crate::{plugin::InheritancePlugin, InheritedElement, InheritedWidget};

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
    C: ContextElements + ContextElement + ContextPlugins<'ctx>,
{
    fn find_inherited_widget<I>(&self) -> Option<Rc<I>>
    where
        I: AnyWidget + InheritedWidget,
    {
        let element_id = self.element_id();

        let Some(inheritance_plugin) = self.plugins().get::<InheritancePlugin>() else {
            tracing::warn!("InheritancePlugin not found in the context");

            return None;
        };

        if let Some(element_id) = inheritance_plugin.find_inherited_element::<I>(element_id) {
            let inherited_element = self
                .elements()
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
    C: ContextElements + ContextElement + ContextPluginsMut<'ctx>,
{
    fn depend_on_inherited_widget<I>(&mut self) -> Option<Rc<I>>
    where
        I: AnyWidget + InheritedWidget,
    {
        let element_id = self.element_id();

        let Some(inheritance_plugin) = self.plugins_mut().get_mut::<InheritancePlugin>() else {
            tracing::warn!("InheritancePlugin not found in the context");

            return None;
        };

        if let Some(element_id) = inheritance_plugin.depend_on_inherited_element::<I>(element_id) {
            let inherited_element = self
                .elements()
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

use std::marker::PhantomData;

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::{Callback, CallbackContext, CallbackFn, CallbackFunc, CallbackId, CallbackQueue},
    element::{Element, ElementId},
    unit::AsAny,
    util::tree::Tree,
    widget::{
        AnyWidget, ContextInheritedMut, ContextWidget, Inheritance, InheritedElement,
        InheritedWidget,
    },
};

pub struct BuildContext<'ctx, W> {
    pub(crate) phantom: PhantomData<W>,

    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,

    pub(crate) callbacks: &'ctx mut FnvHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,

    pub(crate) inheritance: &'ctx mut Inheritance,
}

impl<W> ContextWidget<W> for BuildContext<'_, W> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<W> ContextInheritedMut for BuildContext<'_, W> {
    fn depend_on_inherited_widget<I>(&mut self) -> Option<&I>
    where
        I: AnyWidget + InheritedWidget,
    {
        // We depend on this type, so add it to our set of dependencies even if it doesn't exist
        let Inheritance::Node(inheritance_node) = &mut self.inheritance else {
            panic!("inherited widgets cannot depend on other inherited widgets");
        };

        inheritance_node.add_dependency::<I>();

        if let Some(inheritance_scope_id) = inheritance_node.get_scope() {
            // Grab our ancestor inherited node from the tree. This contains the mappings of available
            // types that we can consume.
            let Inheritance::Scope(target_inheritance_scope) = self
                .element_tree
                .get_mut(inheritance_scope_id)
                .expect("ancestor inherited element does not exist")
                .get_inheritance_mut()
            else {
                panic!("ancestor inherited element was not an inheritance scope")
            };

            // Listen to the inheritance scope we're inheriting relying on for updates
            target_inheritance_scope.add_listener(self.element_id);

            let target_inherited_element_id =
                target_inheritance_scope.find_inherited_widget::<I>(self.element_id)?;

            // Grab the inherited widget we want from the scope
            let target_inherited_element = self
                .element_tree
                .get_mut(target_inherited_element_id)
                .expect("found an inherited widget but it does not exist exist in the tree")
                .downcast_mut::<InheritedElement<I>>()
                .expect("target inherited element downcast failed");

            Some(target_inherited_element.get_inherited_widget())
        } else {
            None
        }
    }
}

impl<W: 'static> BuildContext<'_, W> {
    pub fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }

    pub fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: AsAny,
        F: Fn(&mut CallbackContext<W>, &A) + 'static,
    {
        let callback = Callback::new::<F>(self.element_id, self.callback_queue.clone());

        self.callbacks
            .insert(callback.get_id().unwrap(), Box::new(CallbackFn::new(func)));

        callback
    }
}

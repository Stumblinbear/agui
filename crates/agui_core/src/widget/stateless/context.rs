use std::{any::TypeId, marker::PhantomData};

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::{Callback, CallbackContext, CallbackFn, CallbackFunc, CallbackId, CallbackQueue},
    element::{Element, ElementId},
    unit::Data,
    util::tree::Tree,
    widget::{inheritance::Inheritance, ContextWidget, ContextWidgetMut, InheritedWidget},
};

pub struct BuildContext<'ctx, W> {
    pub(crate) phantom: PhantomData<W>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
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

impl<W: 'static> ContextWidgetMut<W> for BuildContext<'_, W> {
    fn depend_on_inherited_widget<I>(&mut self) -> Option<&mut I>
    where
        I: InheritedWidget + 'static,
    {
        let type_id = TypeId::of::<I>();

        // self.inheritance.listening_to.insert(type_id);

        // if let Some(inheritance_scope_id) = self.inheritance.scope {
        //     // Grab our ancestor inherited node from the tree. This contains the mappings of available
        //     // types that we can consume.
        //     let target_inherited_element_id = self
        //         .element_tree
        //         .get(inheritance_scope_id)
        //         .expect("ancestor inherited node does not exist")
        //         .inheritance_scope
        //         .available
        //         .get(&type_id)
        //         .copied();

        //     if let Some(target_inherited_element_id) = target_inherited_element_id {
        //         let target_inherited_element = self
        //             .element_tree
        //             .get_mut(target_inherited_element_id)
        //             .expect("inherited element does not exist");

        //         let inheritance_node = &mut target_inherited_element.inheritance_scope;

        //         // Add this widget as a listener on the element we're inheriting from
        //         inheritance_node.listeners.insert(self.element_id);

        //         self.inheritance
        //             .depends_on
        //             .insert(target_inherited_element_id);

        //         let instance = target_inherited_element
        //             .downcast_mut::<WidgetInstance<I>>()
        //             .expect("inherited widget downcast failed");

        //         return Some(&mut instance.state);
        //     }
        // }

        None
    }

    fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: Data,
        F: Fn(&mut CallbackContext<W>, &A) + 'static,
    {
        let callback = Callback::new::<F, W>(self.element_id, self.callback_queue.clone());

        self.callbacks
            .insert(callback.get_id().unwrap(), Box::new(CallbackFn::new(func)));

        callback
    }
}

impl<W> BuildContext<'_, W> {
    pub fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

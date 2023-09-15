use std::{any::TypeId, marker::PhantomData, rc::Rc};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    callback::{
        Callback, CallbackContext, CallbackFn, CallbackFunc, CallbackId, CallbackQueue,
        WidgetCallback,
    },
    element::{Element, ElementId},
    inheritance::manager::InheritanceManager,
    unit::AsAny,
    util::tree::Tree,
    widget::{AnyWidget, ContextInheritedMut, ContextWidget, InheritedElement, InheritedWidget},
};

pub struct BuildContext<'ctx, W> {
    pub(crate) phantom: PhantomData<W>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) inheritance_manager: &'ctx mut InheritanceManager,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,

    pub(crate) callbacks: &'ctx mut FxHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,
}

impl<W> ContextWidget for BuildContext<'_, W> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<W> ContextInheritedMut for BuildContext<'_, W> {
    fn depend_on_inherited_widget<I>(&mut self) -> Option<Rc<I>>
    where
        I: AnyWidget + InheritedWidget,
    {
        if let Some(element_id) = self
            .inheritance_manager
            .depend_on_inherited_element(self.element_id, TypeId::of::<I>())
        {
            let inherited_element = self
                .element_tree
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

impl<W: 'static> BuildContext<'_, W> {
    pub fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }

    pub fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: AsAny,
        F: Fn(&mut CallbackContext, A) + 'static,
    {
        let callback = WidgetCallback::new::<F>(self.element_id, self.callback_queue.clone());

        self.callbacks
            .insert(callback.get_id(), Box::new(CallbackFn::new(func)));

        Callback::Widget(callback)
    }
}

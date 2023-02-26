use std::{
    any::TypeId,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::{Callback, CallbackContext, CallbackFn, CallbackFunc, CallbackId, CallbackQueue},
    element::{Element, ElementId},
    inheritance::Inheritance,
    unit::{Data, Key},
    util::tree::Tree,
    widget::{InheritedWidget, IntoElementWidget, WidgetKey, WidgetRef, WidgetState, WidgetView},
};

use super::{ContextStatefulWidget, ContextWidget, ContextWidgetMut};

pub struct BuildContext<'ctx, W>
where
    W: WidgetView,
{
    pub(crate) phantom: PhantomData<W>,

    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,

    pub(crate) inheritance: &'ctx mut Inheritance,

    pub(crate) state: &'ctx mut dyn Data,

    pub(crate) callbacks: &'ctx mut FnvHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,

    pub(crate) keyed_children: FnvHashSet<Key>,
}

impl<W> ContextWidget for BuildContext<'_, W>
where
    W: WidgetView,
{
    type Widget = W;

    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<W> ContextStatefulWidget for BuildContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    type Widget = W;

    fn get_state(&self) -> &W::State {
        self.state.downcast_ref().unwrap()
    }

    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut W::State),
    {
        func(self.state.downcast_mut().unwrap());
    }
}

impl<W> ContextWidgetMut for BuildContext<'_, W>
where
    W: WidgetView,
{
    fn depend_on_inherited_widget<I>(&mut self) -> Option<&mut <I as WidgetState>::State>
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

impl<W> BuildContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    pub fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }

    pub fn key<C>(&mut self, key: Key, widget: C) -> WidgetRef
    where
        C: IntoElementWidget,
    {
        if self.keyed_children.contains(&key) {
            panic!("cannot use the same key twice in a widget");
        }

        self.keyed_children.insert(key);

        WidgetRef::new_with_key(
            Some(match key {
                Key::Local(_) => WidgetKey(Some(self.element_id), key),
                Key::Global(_) => WidgetKey(None, key),
            }),
            widget,
        )
    }
}

impl<W> Deref for BuildContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    type Target = W::State;

    fn deref(&self) -> &Self::Target {
        self.state.downcast_ref().unwrap()
    }
}

impl<W> DerefMut for BuildContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state.downcast_mut().unwrap()
    }
}

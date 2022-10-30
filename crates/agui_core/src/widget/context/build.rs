use std::{any::TypeId, ops::Deref};

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::{Callback, CallbackContext, CallbackFn, CallbackFunc, CallbackId, CallbackQueue},
    context::ContextMut,
    element::{Element, ElementId},
    inheritance::Inheritance,
    unit::{Data, Key},
    util::tree::Tree,
    widget::{InheritedWidget, Widget, WidgetKey, WidgetRef, WidgetState, WidgetView},
};

use super::{ContextStatefulWidget, ContextWidget, ContextWidgetMut};

pub struct BuildContext<'ctx, W>
where
    W: Widget,
{
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,

    pub(crate) inheritance: &'ctx mut Inheritance,

    pub widget: &'ctx W,
    pub state: &'ctx mut W::State,

    pub(crate) callbacks: FnvHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,

    pub(crate) keyed_children: FnvHashSet<Key>,
}

impl<W> Deref for BuildContext<'_, W>
where
    W: Widget,
{
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.widget
    }
}

impl<W> ContextMut for BuildContext<'_, W>
where
    W: Widget,
{
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }

    fn call<A>(&mut self, callback: &Callback<A>, arg: A)
    where
        A: Data,
    {
        self.callback_queue.call(callback, arg);
    }

    fn call_unchecked(&mut self, callback_id: CallbackId, arg: Box<dyn Data>) {
        self.callback_queue.call_unchecked(callback_id, arg);
    }

    fn call_many<A>(&mut self, callbacks: &[Callback<A>], arg: A)
    where
        A: Data,
    {
        self.callback_queue.call_many(callbacks, arg);
    }

    fn call_many_unchecked(&mut self, callback_ids: &[CallbackId], arg: Box<dyn Data>) {
        self.callback_queue.call_many_unchecked(callback_ids, arg);
    }
}

impl<W> ContextWidget for BuildContext<'_, W>
where
    W: Widget,
{
    type Widget = W;

    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }

    fn get_widget(&self) -> &W {
        self.widget
    }
}

impl<W> ContextStatefulWidget for BuildContext<'_, W>
where
    W: Widget,
{
    fn get_state(&self) -> &W::State {
        self.state
    }

    fn get_state_mut(&mut self) -> &mut W::State {
        self.state
    }

    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut W::State),
    {
        func(self.state);
    }
}

impl<W> ContextWidgetMut for BuildContext<'_, W>
where
    W: Widget,
{
    fn depend_on_inherited_widget<I>(&mut self) -> Option<&mut I::State>
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
    pub fn key<C>(&mut self, key: Key, widget: C) -> WidgetRef
    where
        C: Widget,
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

use std::{any::TypeId, marker::PhantomData};

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::{Callback, CallbackId, CallbackQueue},
    element::{Element, ElementId},
    unit::{Data, Key},
    util::tree::Tree,
    widget::{
        AnyWidget, ContextInheritedMut, ContextWidget, Inheritance, InheritedWidget, WidgetKey,
        WidgetRef, WidgetState,
    },
};

use super::{ContextWidgetState, StatefulCallbackContext};

pub trait StatefulCallbackFunc<W> {
    #[allow(clippy::borrowed_box)]
    fn call(&self, ctx: &mut StatefulCallbackContext<W>, args: &Box<dyn Data>);
}

pub struct StatefulCallbackFn<W, A, F>
where
    A: 'static,
    F: Fn(&mut StatefulCallbackContext<W>, &A),
{
    phantom: PhantomData<(W, A, F)>,

    func: F,
}

impl<W, A, F> StatefulCallbackFn<W, A, F>
where
    A: 'static,
    F: Fn(&mut StatefulCallbackContext<W>, &A),
{
    pub fn new(func: F) -> Self {
        Self {
            phantom: PhantomData,

            func,
        }
    }
}

impl<W, A, F> StatefulCallbackFunc<W> for StatefulCallbackFn<W, A, F>
where
    A: Data,
    F: Fn(&mut StatefulCallbackContext<W>, &A),
{
    fn call(&self, ctx: &mut StatefulCallbackContext<W>, args: &Box<dyn Data>) {
        let args = args
            .downcast_ref::<A>()
            .expect("failed to downcast callback args");

        (self.func)(ctx, args)
    }
}

pub struct StatefulContext<'ctx, S>
where
    S: WidgetState,
{
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,

    pub(crate) callbacks: &'ctx mut FnvHashMap<CallbackId, Box<dyn StatefulCallbackFunc<S>>>,

    pub(crate) inheritance: &'ctx mut Inheritance,

    pub(crate) keyed_children: FnvHashSet<Key>,

    pub widget: &'ctx S::Widget,
    pub(crate) state: &'ctx S,
}

impl<S> ContextWidget<S> for StatefulContext<'_, S>
where
    S: WidgetState,
{
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<S> ContextWidgetState<S> for StatefulContext<'_, S>
where
    S: WidgetState,
{
    fn get_state(&self) -> &S {
        self.state
    }
}

impl<'ctx, S: 'static> StatefulContext<'ctx, S>
where
    S: WidgetState,
{
    pub fn get_widget(&self) -> &S::Widget {
        self.widget
    }

    pub fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }

    pub fn key<C>(&mut self, key: Key, widget: C) -> WidgetRef
    where
        C: AnyWidget,
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

    pub fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: Data,
        F: Fn(&mut StatefulCallbackContext<S>, &A) + 'static,
    {
        let callback = Callback::new::<F, S>(self.element_id, self.callback_queue.clone());

        self.callbacks.insert(
            callback.get_id().unwrap(),
            Box::new(StatefulCallbackFn::new(func)),
        );

        callback
    }
}

impl<S> ContextInheritedMut for StatefulContext<'_, S>
where
    S: WidgetState,
{
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
}

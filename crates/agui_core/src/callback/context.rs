use std::ops::Deref;

use fnv::FnvHashSet;

use crate::{
    context::ContextMut,
    element::{Element, ElementId},
    unit::Data,
    util::tree::Tree,
    widget::{ContextStatefulWidget, ContextWidget, Widget},
};

use super::{Callback, CallbackId, CallbackQueue};

pub struct CallbackContext<'ctx, W>
where
    W: Widget,
{
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,
    pub widget: &'ctx W,
    pub state: &'ctx mut W::State,

    pub(crate) changed: bool,
}

impl<W> Deref for CallbackContext<'_, W>
where
    W: Widget,
{
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.widget
    }
}

impl<W> ContextMut for CallbackContext<'_, W>
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

impl<W> ContextWidget for CallbackContext<'_, W>
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

impl<W> ContextStatefulWidget for CallbackContext<'_, W>
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
        self.changed = true;

        func(self.state);
    }
}

use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use fnv::FnvHashSet;

use crate::{
    element::{Element, ElementId},
    unit::Data,
    util::tree::Tree,
    widget::{WidgetState, WidgetView},
};

use super::{ContextStatefulWidget, ContextWidget};

pub struct LayoutContext<'ctx, W>
where
    W: WidgetView,
{
    pub(crate) phantom: PhantomData<W>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,

    pub(crate) element_id: ElementId,

    pub(crate) state: &'ctx mut dyn Data,
}

impl<W> ContextWidget for LayoutContext<'_, W>
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

impl<W> ContextStatefulWidget for LayoutContext<'_, W>
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

impl<W> LayoutContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    pub fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

impl<W> Deref for LayoutContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    type Target = W::State;

    fn deref(&self) -> &Self::Target {
        self.state.downcast_ref().unwrap()
    }
}

impl<W> DerefMut for LayoutContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state.downcast_mut().unwrap()
    }
}

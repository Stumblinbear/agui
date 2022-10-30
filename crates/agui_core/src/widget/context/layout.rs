use std::ops::Deref;

use crate::{
    element::{Element, ElementId},
    util::tree::Tree,
    widget::Widget,
};

use super::{ContextStatefulWidget, ContextWidget};

pub struct LayoutContext<'ctx, W>
where
    W: Widget,
{
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,
    pub widget: &'ctx W,
    pub state: &'ctx mut W::State,
}

impl<W> Deref for LayoutContext<'_, W>
where
    W: Widget,
{
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.widget
    }
}

impl<W> ContextWidget for LayoutContext<'_, W>
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

impl<W> ContextStatefulWidget for LayoutContext<'_, W>
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

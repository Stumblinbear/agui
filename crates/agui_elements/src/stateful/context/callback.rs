use std::ops::{Deref, DerefMut};

use agui_core::element::{ContextElements, ElementCallbackContext};

use agui_core::{
    element::{ContextDirtyElement, ContextElement, Element, ElementId},
    util::tree::Tree,
};

use crate::stateful::WidgetState;

use super::ContextWidgetStateMut;

pub struct StatefulCallbackContext<'ctx, 'element, S>
where
    S: WidgetState + ?Sized,
{
    pub(crate) inner: &'element mut ElementCallbackContext<'ctx>,

    pub widget: &'element S::Widget,

    pub state: &'element mut S,
    pub(crate) is_changed: bool,
}

impl<S> ContextElements for StatefulCallbackContext<'_, '_, S>
where
    S: WidgetState + ?Sized,
{
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.inner.elements()
    }
}

impl<S> ContextElement for StatefulCallbackContext<'_, '_, S>
where
    S: WidgetState + ?Sized,
{
    fn element_id(&self) -> ElementId {
        self.inner.element_id()
    }
}

impl<S> ContextDirtyElement for StatefulCallbackContext<'_, '_, S>
where
    S: WidgetState + ?Sized,
{
    fn mark_needs_build(&mut self) {
        self.inner.mark_needs_build();
    }
}

impl<'ctx, S> ContextWidgetStateMut<'ctx, S> for StatefulCallbackContext<'ctx, '_, S>
where
    S: WidgetState + ?Sized,
{
    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut S),
    {
        func(self.state);

        self.is_changed = true;
    }
}

impl<'ctx, S: 'static> Deref for StatefulCallbackContext<'ctx, '_, S>
where
    S: WidgetState,
{
    type Target = ElementCallbackContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'ctx, S: 'static> DerefMut for StatefulCallbackContext<'ctx, '_, S>
where
    S: WidgetState,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

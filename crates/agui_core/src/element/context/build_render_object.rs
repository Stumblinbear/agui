use std::ops::{Deref, DerefMut};

use crate::{
    callback::{CallbackQueue, ContextCallbackQueue},
    element::{Element, ElementBuildContext, ElementId},
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    util::tree::Tree,
};

use super::{ContextElement, ContextElements};

pub struct RenderObjectBuildContext<'ctx, 'element> {
    pub(crate) inner: &'element mut ElementBuildContext<'ctx>,
}

impl ContextElements for RenderObjectBuildContext<'_, '_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.inner.elements()
    }
}

impl ContextElement for RenderObjectBuildContext<'_, '_> {
    fn element_id(&self) -> ElementId {
        self.inner.element_id()
    }
}

impl<'ctx> ContextPlugins<'ctx> for RenderObjectBuildContext<'ctx, '_> {
    fn plugins(&self) -> &Plugins {
        self.inner.plugins()
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for RenderObjectBuildContext<'ctx, '_> {
    fn plugins_mut(&mut self) -> &mut Plugins {
        self.inner.plugins_mut()
    }
}

impl ContextCallbackQueue for RenderObjectBuildContext<'_, '_> {
    fn callback_queue(&self) -> &CallbackQueue {
        self.inner.callback_queue()
    }
}

impl<'ctx> Deref for RenderObjectBuildContext<'ctx, '_> {
    type Target = ElementBuildContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'ctx> DerefMut for RenderObjectBuildContext<'ctx, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

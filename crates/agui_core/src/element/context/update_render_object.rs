use std::ops::{Deref, DerefMut};

use crate::{
    callback::{CallbackQueue, ContextCallbackQueue},
    element::{ContextDirtyRenderObject, Element, ElementBuildContext, ElementId},
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    render::RenderObjectId,
    util::tree::Tree,
};

use super::{ContextElement, ContextElements, ContextRenderObject};

pub struct RenderObjectUpdateContext<'ctx, 'element> {
    pub(crate) inner: &'element mut ElementBuildContext<'ctx>,

    pub render_object_id: &'element RenderObjectId,
}

impl ContextElements for RenderObjectUpdateContext<'_, '_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.inner.elements()
    }
}

impl ContextElement for RenderObjectUpdateContext<'_, '_> {
    fn element_id(&self) -> ElementId {
        self.inner.element_id()
    }
}

impl ContextRenderObject for RenderObjectUpdateContext<'_, '_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}

impl<'ctx> ContextPlugins<'ctx> for RenderObjectUpdateContext<'ctx, '_> {
    fn plugins(&self) -> &Plugins {
        self.inner.plugins()
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for RenderObjectUpdateContext<'ctx, '_> {
    fn plugins_mut(&mut self) -> &mut Plugins {
        self.inner.plugins_mut()
    }
}

impl ContextDirtyRenderObject for RenderObjectUpdateContext<'_, '_> {
    fn mark_needs_layout(&mut self) {
        self.inner.needs_layout.insert(*self.render_object_id)
    }

    fn mark_needs_paint(&mut self) {
        self.inner.needs_paint.insert(*self.render_object_id)
    }
}

impl ContextCallbackQueue for RenderObjectUpdateContext<'_, '_> {
    fn callback_queue(&self) -> &CallbackQueue {
        self.inner.callback_queue()
    }
}

impl<'ctx> Deref for RenderObjectUpdateContext<'ctx, '_> {
    type Target = ElementBuildContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'ctx> DerefMut for RenderObjectUpdateContext<'ctx, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

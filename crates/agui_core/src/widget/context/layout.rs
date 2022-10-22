use std::ops::Deref;

use crate::{
    manager::element::WidgetElement,
    plugin::{BoxedPlugin, PluginElement, PluginId, PluginImpl},
    util::{map::PluginMap, tree::Tree},
    widget::{WidgetId, WidgetState, WidgetView},
};

use super::{ContextPlugins, ContextStatefulWidget, ContextWidget};

pub struct LayoutContext<'ctx, W>
where
    W: WidgetView + WidgetState,
{
    pub(crate) plugins: &'ctx mut PluginMap<BoxedPlugin>,
    pub(crate) widget_tree: &'ctx Tree<WidgetId, WidgetElement>,

    pub(crate) widget_id: WidgetId,
    pub widget: &'ctx W,
    pub state: &'ctx mut W::State,
}

impl<W> Deref for LayoutContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.widget
    }
}

impl<W> ContextPlugins for LayoutContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    fn get_plugins(&mut self) -> &mut PluginMap<BoxedPlugin> {
        self.plugins
    }

    fn get_plugin<P>(&self) -> Option<&PluginElement<P>>
    where
        P: PluginImpl,
    {
        self.plugins
            .get(&PluginId::of::<P>())
            .and_then(|p| p.downcast_ref())
    }

    fn get_plugin_mut<P>(&mut self) -> Option<&mut PluginElement<P>>
    where
        P: PluginImpl,
    {
        self.plugins
            .get_mut(&PluginId::of::<P>())
            .and_then(|p| p.downcast_mut())
    }
}

impl<W> ContextWidget for LayoutContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    type Widget = W;

    fn get_widgets(&self) -> &Tree<WidgetId, WidgetElement> {
        self.widget_tree
    }

    fn get_widget_id(&self) -> WidgetId {
        self.widget_id
    }

    fn get_widget(&self) -> &W {
        self.widget
    }
}

impl<W> ContextStatefulWidget for LayoutContext<'_, W>
where
    W: WidgetView + WidgetState,
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

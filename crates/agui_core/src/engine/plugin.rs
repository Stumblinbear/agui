use std::any::{type_name, TypeId};

use downcast_rs::{impl_downcast, Downcast};

use crate::plugin::{EnginePlugin, PluginContext};

use super::{context::EngineContext, event::WidgetEvent};

pub trait PluginImpl: std::fmt::Debug + Downcast {
    fn get_type_id(&self) -> TypeId;
    fn get_type_name(&self) -> &'static str;

    fn on_before_update(&mut self, ctx: EngineContext);
    fn on_update(&mut self, ctx: EngineContext);
    fn on_layout(&mut self, ctx: EngineContext);
    fn on_events(&mut self, ctx: EngineContext, events: &[WidgetEvent]);
}

impl_downcast!(PluginImpl);

#[derive(Default)]
pub struct PluginElement<P>
where
    P: EnginePlugin,
{
    plugin: P,
    state: P::State,
}

impl<P> PluginElement<P>
where
    P: EnginePlugin,
{
    pub fn new(plugin: P) -> Self {
        Self {
            plugin,
            state: P::State::default(),
        }
    }
}

impl<P> PluginElement<P>
where
    P: EnginePlugin,
{
    pub fn get_plugin(&self) -> &P {
        &self.plugin
    }

    pub fn get_state(&self) -> &P::State {
        &self.state
    }

    pub fn get_state_mut(&mut self) -> &mut P::State {
        &mut self.state
    }
}

impl<P> PluginImpl for PluginElement<P>
where
    P: EnginePlugin,
{
    fn get_type_id(&self) -> TypeId {
        TypeId::of::<P>()
    }

    fn get_type_name(&self) -> &'static str {
        type_name::<P>()
    }

    fn on_before_update(&mut self, ctx: EngineContext) {
        let mut ctx = PluginContext {
            tree: ctx.tree,
            dirty: ctx.dirty,
            notifier: ctx.notifier,
        };

        self.plugin.on_before_update(&mut ctx, &mut self.state);
    }

    fn on_update(&mut self, ctx: EngineContext) {
        let mut ctx = PluginContext {
            tree: ctx.tree,
            dirty: ctx.dirty,
            notifier: ctx.notifier,
        };

        self.plugin.on_update(&mut ctx, &mut self.state);
    }

    fn on_layout(&mut self, ctx: EngineContext) {
        let mut ctx = PluginContext {
            tree: ctx.tree,
            dirty: ctx.dirty,
            notifier: ctx.notifier,
        };

        self.plugin.on_layout(&mut ctx, &mut self.state);
    }

    fn on_events(&mut self, ctx: EngineContext, events: &[WidgetEvent]) {
        let mut ctx = PluginContext {
            tree: ctx.tree,
            dirty: ctx.dirty,
            notifier: ctx.notifier,
        };

        self.plugin.on_events(&mut ctx, &mut self.state, events);
    }
}

impl<P> std::fmt::Debug for PluginElement<P>
where
    P: EnginePlugin,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginElement")
            .field("plugin", &self.plugin)
            .field("state", &self.state)
            .finish()
    }
}

impl<P> From<P> for PluginElement<P>
where
    P: EnginePlugin,
{
    fn from(plugin: P) -> Self {
        Self::new(plugin)
    }
}

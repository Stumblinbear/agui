use std::any::{type_name, TypeId};

use crate::manager::{context::AguiContext, events::WidgetEvent};

use super::{PluginContext, PluginImpl, PluginInstance};

#[derive(Default)]
pub struct PluginElement<P>
where
    P: PluginImpl,
{
    plugin: P,
    state: P::State,
}

impl<P> PluginElement<P>
where
    P: PluginImpl,
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
    P: PluginImpl,
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

impl<P> PluginInstance for PluginElement<P>
where
    P: PluginImpl,
{
    fn get_type_id(&self) -> TypeId {
        TypeId::of::<P>()
    }

    fn get_display_name(&self) -> String {
        let type_name = type_name::<P>();

        if !type_name.contains('<') {
            String::from(type_name.rsplit("::").next().unwrap())
        } else {
            let mut name = String::new();

            let mut remaining = String::from(type_name);

            while let Some((part, rest)) = remaining.split_once('<') {
                name.push_str(part.rsplit("::").next().unwrap());

                name.push('<');

                remaining = String::from(rest);
            }

            name.push_str(remaining.rsplit("::").next().unwrap());

            name
        }
    }

    fn on_before_update(&mut self, ctx: crate::manager::context::AguiContext) {
        let span = tracing::error_span!("on_before_update");
        let _enter = span.enter();

        let mut ctx = PluginContext {
            tree: ctx.tree,
            dirty: ctx.dirty,

            callback_queue: ctx.callback_queue,
        };

        self.plugin.on_before_update(&mut ctx, &mut self.state);
    }

    fn on_update(&mut self, ctx: AguiContext) {
        let span = tracing::error_span!("on_update");
        let _enter = span.enter();

        let mut ctx = PluginContext {
            tree: ctx.tree,
            dirty: ctx.dirty,

            callback_queue: ctx.callback_queue,
        };

        self.plugin.on_update(&mut ctx, &mut self.state);
    }

    fn on_layout(&mut self, ctx: AguiContext) {
        let span = tracing::error_span!("on_layout");
        let _enter = span.enter();

        let mut ctx = PluginContext {
            tree: ctx.tree,
            dirty: ctx.dirty,

            callback_queue: ctx.callback_queue,
        };

        self.plugin.on_layout(&mut ctx, &mut self.state);
    }

    fn on_events(&mut self, ctx: AguiContext, events: &[WidgetEvent]) {
        let span = tracing::error_span!("on_events");
        let _enter = span.enter();

        let mut ctx = PluginContext {
            tree: ctx.tree,
            dirty: ctx.dirty,

            callback_queue: ctx.callback_queue,
        };

        self.plugin.on_events(&mut ctx, &mut self.state, events);
    }
}

impl<P> std::fmt::Debug for PluginElement<P>
where
    P: PluginImpl + std::fmt::Debug,
    <P>::State: std::fmt::Debug,
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
    P: PluginImpl,
{
    fn from(plugin: P) -> Self {
        Self::new(plugin)
    }
}

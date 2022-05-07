use std::{
    any::{type_name, TypeId},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::plugin::{PluginContext, PluginImpl, WidgetManagerPlugin};

use super::{context::AguiContext, event::WidgetEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PluginId(TypeId);

impl PluginId {
    pub fn of<P>() -> Self
    where
        P: WidgetManagerPlugin,
    {
        Self(TypeId::of::<P>())
    }
}

#[derive(Debug)]
pub struct Plugin(Box<dyn PluginImpl>);

impl Plugin {
    pub(crate) fn new<P>(plugin: P) -> Self
    where
        P: PluginImpl,
    {
        Self(Box::new(plugin))
    }

    #[allow(clippy::borrowed_box)]
    pub fn get(&self) -> &Box<dyn PluginImpl> {
        &self.0
    }

    pub fn get_mut(&mut self) -> &mut Box<dyn PluginImpl> {
        &mut self.0
    }

    pub fn get_as<P>(&self) -> Option<PluginRef<P>>
    where
        P: WidgetManagerPlugin,
    {
        if self.0.get_type_id() == TypeId::of::<P>() {
            Some(PluginRef {
                phantom: PhantomData,

                plugin: &self.0,
            })
        } else {
            None
        }
    }

    pub fn get_as_mut<P>(&mut self) -> Option<PluginMut<P>>
    where
        P: WidgetManagerPlugin,
    {
        if self.0.get_type_id() == TypeId::of::<P>() {
            Some(PluginMut {
                phantom: PhantomData,

                plugin: &mut self.0,
            })
        } else {
            None
        }
    }
}

impl Deref for Plugin {
    type Target = Box<dyn PluginImpl>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Plugin {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct PluginRef<'b, P>
where
    P: WidgetManagerPlugin,
{
    phantom: PhantomData<P>,

    #[allow(clippy::borrowed_box)]
    plugin: &'b Box<dyn PluginImpl>,
}

impl<'b, P> Deref for PluginRef<'b, P>
where
    P: WidgetManagerPlugin,
{
    type Target = PluginElement<P>;

    fn deref(&self) -> &Self::Target {
        self.plugin
            .downcast_ref::<PluginElement<P>>()
            .expect("invalid PluginRef created")
    }
}

pub struct PluginMut<'b, P>
where
    P: WidgetManagerPlugin,
{
    phantom: PhantomData<P>,

    plugin: &'b mut Box<dyn PluginImpl>,
}

impl<'b, P> Deref for PluginMut<'b, P>
where
    P: WidgetManagerPlugin,
{
    type Target = PluginElement<P>;

    fn deref(&self) -> &Self::Target {
        self.plugin
            .downcast_ref::<PluginElement<P>>()
            .expect("invalid PluginRef created")
    }
}

impl<'b, P> DerefMut for PluginMut<'b, P>
where
    P: WidgetManagerPlugin,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.plugin
            .downcast_mut::<PluginElement<P>>()
            .expect("invalid PluginRef created")
    }
}

#[derive(Default)]
pub struct PluginElement<P>
where
    P: WidgetManagerPlugin,
{
    plugin: P,
    state: P::State,
}

impl<P> PluginElement<P>
where
    P: WidgetManagerPlugin,
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
    P: WidgetManagerPlugin,
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
    P: WidgetManagerPlugin,
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

            while let Some((part, rest)) = remaining.split_once("<") {
                name.push_str(part.rsplit("::").next().unwrap());

                name.push('<');

                remaining = String::from(rest);
            }

            name.push_str(remaining.rsplit("::").next().unwrap());

            name
        }
    }

    fn on_before_update(&mut self, ctx: AguiContext) {
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
    P: WidgetManagerPlugin,
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
    P: WidgetManagerPlugin,
{
    fn from(plugin: P) -> Self {
        Self::new(plugin)
    }
}

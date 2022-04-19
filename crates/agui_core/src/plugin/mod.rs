use std::{
    any::TypeId,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use downcast_rs::Downcast;

use crate::manager::{
    event::WidgetEvent,
    plugin::{PluginImpl, PluginElement},
    Data,
};

mod context;

pub use context::*;

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

/// A plugin for the widget manager.
#[allow(unused_variables)]
pub trait WidgetManagerPlugin: std::fmt::Debug + Downcast {
    type State: Data + Default;

    /// Fired every time the widget manager is updated, before any widgets are updated.
    fn on_before_update(&self, ctx: &mut PluginContext, state: &mut Self::State) {}

    /// Fired every time the widget manager is updated, after all widgets are updated.
    fn on_update(&self, ctx: &mut PluginContext, state: &mut Self::State) {}

    /// Fired after widgets are updated, just after the layout is resolved.
    ///
    /// This may listen to changes, however it's fired following the layout being resolved, meaning
    /// it has up-to-date information on real widget size. This may listen and react to state, but if
    /// possible it should only modify state if absolutely necessary because any update notifications
    /// will cause the layout to be recalculated.
    fn on_layout(&self, ctx: &mut PluginContext, state: &mut Self::State) {}

    /// Allows the plugin to listen to widget tree events.
    fn on_events(&self, ctx: &mut PluginContext, state: &mut Self::State, events: &[WidgetEvent]) {}
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

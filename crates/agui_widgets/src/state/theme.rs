use std::{any::TypeId, collections::BTreeMap};

use agui_core::widget::BuildContext;
use downcast_rs::{impl_downcast, Downcast};

use crate::plugins::provider::ConsumerExt;

pub trait Style: std::fmt::Debug + Downcast + Send + Sync {}

impl<T> Style for T where T: std::fmt::Debug + Downcast + Send + Sync {}

impl_downcast!(Style);

#[derive(Debug, Default)]
pub struct Theme {
    styles: BTreeMap<TypeId, Box<dyn Style>>,
}

impl Theme {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set<S>(&mut self, style: S)
    where
        S: Style,
    {
        self.styles.insert(TypeId::of::<S>(), Box::new(style));
    }

    pub fn get<S>(&self) -> Option<&S>
    where
        S: Style,
    {
        let style_id = TypeId::of::<S>();

        self.styles
            .get(&style_id)
            .map(|s| s.downcast_ref::<S>().expect("failed to downcast style"))
    }

    pub fn get_or_init<S>(&self) -> S
    where
        S: Style + Clone + Default,
    {
        if let Some(style) = self.get::<S>() {
            style.clone()
        } else {
            S::default()
        }
    }

    pub fn resolve<S>(ctx: &mut BuildContext, style: Option<&S>) -> S
    where
        S: Style + Clone + Default,
    {
        if let Some(style) = style {
            style.clone()
        // This either grabs a provided theme or uses global state
        } else if let Some(theme) = ctx.consume::<Theme>() {
            theme.read().get_or_init::<S>()
        } else {
            S::default()
        }
    }
}

pub trait StyleExt<S>
where
    S: Style + Clone + Default,
{
    fn resolve(&self, ctx: &mut BuildContext) -> S;
}

impl<S> StyleExt<S> for S
where
    S: Style + Clone + Default,
{
    fn resolve(&self, ctx: &mut BuildContext) -> S {
        Theme::resolve(ctx, Some(self))
    }
}

impl<S> StyleExt<S> for Option<S>
where
    S: Style + Clone + Default,
{
    fn resolve(&self, ctx: &mut BuildContext) -> S {
        Theme::resolve(ctx, self.as_ref())
    }
}

impl<S> StyleExt<S> for Option<&S>
where
    S: Style + Clone + Default,
{
    fn resolve(&self, ctx: &mut BuildContext) -> S {
        Theme::resolve(ctx, *self)
    }
}

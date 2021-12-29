use std::{any::TypeId, collections::BTreeMap};

use agui_core::context::WidgetContext;
use downcast_rs::{impl_downcast, Downcast};

pub trait Style: Downcast + Send + Sync {}

impl_downcast!(Style);

pub struct Theme {
    styles: BTreeMap<TypeId, Box<dyn Style>>,
}

impl Theme {
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

    pub fn resolve<S>(ctx: &WidgetContext, style: &Option<S>) -> S
    where
        S: Style + Clone + Default,
    {
        if let Some(style) = style {
            style.clone()
        } else if let Some(theme) = ctx.get_global::<Theme>() {
            theme.read().get_or_init::<S>()
        } else {
            S::default()
        }
    }
}

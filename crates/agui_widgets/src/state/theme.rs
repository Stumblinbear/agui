use std::{any::TypeId, collections::BTreeMap, rc::Rc};

use agui_core::{manager::Data, widget::BuildContext};

#[derive(Debug, Default, Clone)]
pub struct Theme {
    styles: BTreeMap<TypeId, Rc<dyn Data>>,
}

impl Theme {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set<S>(&mut self, style: S)
    where
        S: Data,
    {
        self.styles.insert(TypeId::of::<S>(), Rc::new(style));
    }

    pub fn get<S>(&self) -> Option<S>
    where
        S: Data + Clone,
    {
        let style_id = TypeId::of::<S>();

        self.styles.get(&style_id).map(|s| {
            Rc::clone(s)
                .downcast::<S>()
                .expect("failed to downcast style")
                .as_ref()
                .clone()
        })
    }

    pub fn get_or_init<S>(&self) -> S
    where
        S: Data + Clone + Default,
    {
        self.get::<S>().unwrap_or_default()
    }

    pub fn resolve<S>(ctx: &mut BuildContext, style: Option<&S>) -> S
    where
        S: Data + Clone + Default,
    {
        if let Some(style) = style {
            style.clone()
        // This either grabs a provided theme or uses global state
        } else if let Some(theme) = ctx.consume::<Theme>() {
            theme.get_or_init::<S>()
        } else {
            S::default()
        }
    }
}

pub trait StyleExt<S>
where
    S: Data + Clone + Default,
{
    fn resolve(&self, ctx: &mut BuildContext) -> S;
}

impl<S> StyleExt<S> for S
where
    S: Data + Clone + Default,
{
    fn resolve(&self, ctx: &mut BuildContext) -> S {
        Theme::resolve(ctx, Some(self))
    }
}

impl<S> StyleExt<S> for Option<S>
where
    S: Data + Clone + Default,
{
    fn resolve(&self, ctx: &mut BuildContext) -> S {
        Theme::resolve(ctx, self.as_ref())
    }
}

impl<S> StyleExt<S> for Option<&S>
where
    S: Data + Clone + Default,
{
    fn resolve(&self, ctx: &mut BuildContext) -> S {
        Theme::resolve(ctx, *self)
    }
}

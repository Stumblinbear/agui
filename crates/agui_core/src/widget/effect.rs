use std::marker::PhantomData;

use crate::widget::WidgetContext;

pub trait EffectFunc<'ui> {
    fn call(&self, ctx: &mut WidgetContext<'ui, '_>);
}

pub struct EffectFn<'ui, F>
where
    F: Fn(&mut WidgetContext<'ui, '_>),
{
    phantom: PhantomData<&'ui F>,

    func: F,
}

impl<'ui, F> EffectFn<'ui, F>
where
    F: Fn(&mut WidgetContext<'ui, '_>),
{
    pub fn new(func: F) -> Self {
        Self {
            phantom: PhantomData,
            
            func,
        }
    }
}

impl<'ui, F> EffectFunc<'ui> for EffectFn<'ui, F>
where
    F: Fn(&mut WidgetContext<'ui, '_>),
{
    fn call(&self, ctx: &mut WidgetContext<'ui, '_>) {
        (self.func)(ctx)
    }
}

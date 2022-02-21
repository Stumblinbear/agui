use std::marker::PhantomData;

use crate::{state::StateValue, widget::WidgetContext};

pub trait ComputedFunc<'ui> {
    fn call(&mut self, ctx: &mut WidgetContext<'ui, '_>) -> bool;

    fn get(&self) -> Box<dyn StateValue>;

    fn did_change(&self) -> bool;
}

pub struct ComputedFn<'ui, V, F>
where
    V: Eq + PartialEq + Clone + StateValue,
    F: Fn(&mut WidgetContext<'ui, '_>) -> V,
{
    phantom: PhantomData<&'ui V>,

    did_change: bool,

    value: Option<V>,
    func: F,
}

impl<'ui, V, F> ComputedFn<'ui, V, F>
where
    V: Eq + PartialEq + Clone + StateValue,
    F: Fn(&mut WidgetContext<'ui, '_>) -> V,
{
    pub fn new(func: F) -> Self {
        Self {
            phantom: PhantomData,

            did_change: false,

            value: None,
            func,
        }
    }
}

impl<'ui, V, F> ComputedFunc<'ui> for ComputedFn<'ui, V, F>
where
    V: Eq + PartialEq + Clone + StateValue,
    F: Fn(&mut WidgetContext<'ui, '_>) -> V,
{
    fn call(&mut self, ctx: &mut WidgetContext<'ui, '_>) -> bool {
        let new_value = (self.func)(ctx);

        self.did_change = match &self.value {
            Some(old_value) => *old_value != new_value,
            None => true,
        };

        self.value = Some(new_value);

        self.did_change
    }

    fn get(&self) -> Box<dyn StateValue> {
        Box::new(self.value.as_ref().unwrap().clone())
    }

    fn did_change(&self) -> bool {
        self.did_change
    }
}

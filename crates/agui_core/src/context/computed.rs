use std::marker::PhantomData;

use super::{ListenerId, NotifiableValue, WidgetContext};

pub trait ComputedFunc<'ui> {
    fn call(&mut self, ctx: &WidgetContext<'ui>) -> bool;

    fn get(&self) -> Box<dyn NotifiableValue>;

    fn did_change(&self) -> bool;
}

pub struct ComputedFn<'ui, V, F>
where
    V: Eq + PartialEq + Clone + NotifiableValue,
    F: Fn(&WidgetContext<'ui>) -> V,
{
    phantom: PhantomData<&'ui V>,

    listener_id: ListenerId,
    did_change: bool,

    value: Option<V>,
    func: F,
}

impl<'ui, V, F> ComputedFn<'ui, V, F>
where
    V: Eq + PartialEq + Clone + NotifiableValue,
    F: Fn(&WidgetContext<'ui>) -> V,
{
    pub fn new(listener_id: ListenerId, func: F) -> Self {
        Self {
            phantom: PhantomData,

            listener_id,
            did_change: false,

            value: None,
            func,
        }
    }
}

impl<'ui, V, F> ComputedFunc<'ui> for ComputedFn<'ui, V, F>
where
    V: Eq + PartialEq + Clone + NotifiableValue,
    F: Fn(&WidgetContext<'ui>) -> V,
{
    fn call(&mut self, ctx: &WidgetContext<'ui>) -> bool {
        let new_value = {
            let previous_id = *ctx.current_id.lock();

            *ctx.current_id.lock() = Some(self.listener_id);

            let value = (self.func)(ctx);

            *ctx.current_id.lock() = previous_id;

            value
        };

        self.did_change = match &self.value {
            Some(old_value) => *old_value != new_value,
            None => true,
        };

        self.value = Some(new_value);

        self.did_change
    }

    fn get(&self) -> Box<dyn NotifiableValue> {
        Box::new(self.value.as_ref().unwrap().clone())
    }

    fn did_change(&self) -> bool {
        self.did_change
    }
}

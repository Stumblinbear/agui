use std::{any::TypeId, marker::PhantomData};

use crate::notifiable::NotifiableValue;

mod context;

pub use context::ComputedContext;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct ComputedId(TypeId);

impl ComputedId {
    pub fn of<F>() -> Self
    where
        F: ?Sized + 'static,
    {
        Self(TypeId::of::<F>())
    }
}

pub trait ComputedFunc<'ui> {
    fn call(&mut self, ctx: &mut ComputedContext<'ui, '_>) -> bool;

    fn get(&self) -> Box<dyn NotifiableValue>;

    fn did_change(&self) -> bool;
}

pub struct ComputedFn<'ui, V, F>
where
    V: Eq + PartialEq + Clone + NotifiableValue,
    F: Fn(&mut ComputedContext<'ui, '_>) -> V,
{
    phantom: PhantomData<&'ui V>,

    did_change: bool,

    value: Option<V>,
    func: F,
}

impl<'ui, V, F> ComputedFn<'ui, V, F>
where
    V: Eq + PartialEq + Clone + NotifiableValue,
    F: Fn(&mut ComputedContext<'ui, '_>) -> V,
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
    V: Eq + PartialEq + Clone + NotifiableValue,
    F: Fn(&mut ComputedContext<'ui, '_>) -> V,
{
    fn call(&mut self, ctx: &mut ComputedContext<'ui, '_>) -> bool {
        let new_value = (self.func)(ctx);

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

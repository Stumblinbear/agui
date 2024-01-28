use std::{any::Any, marker::PhantomData};

use crate::stateful::WidgetState;

use super::StatefulCallbackContext;

pub trait StatefulCallbackFunc<S>
where
    S: WidgetState,
{
    fn call(&self, ctx: &mut StatefulCallbackContext<S>, args: Box<dyn Any>);
}

pub struct StatefulCallbackFn<S, A, F> {
    phantom: PhantomData<(S, A, F)>,

    func: F,
}

impl<S, A, F> StatefulCallbackFn<S, A, F>
where
    F: Fn(&mut StatefulCallbackContext<S>, A),
{
    pub fn new(func: F) -> Self {
        Self {
            phantom: PhantomData,

            func,
        }
    }
}

impl<S, A, F> StatefulCallbackFunc<S> for StatefulCallbackFn<S, A, F>
where
    S: WidgetState,
    A: Any,
    F: Fn(&mut StatefulCallbackContext<S>, A),
{
    fn call(&self, ctx: &mut StatefulCallbackContext<S>, arg: Box<dyn Any>) {
        let arg = arg
            .downcast::<A>()
            .expect("failed to downcast callback argument");

        (self.func)(ctx, *arg)
    }
}

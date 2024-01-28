use std::{any::Any, marker::PhantomData};

use super::StatelessCallbackContext;

pub trait StatelessCallbackFunc<W> {
    fn call(&self, ctx: &mut StatelessCallbackContext, args: Box<dyn Any>);
}

pub struct StatelessCallbackFn<W, A, F>
where
    A: Any,
    F: Fn(&mut StatelessCallbackContext, A),
{
    phantom: PhantomData<(W, A, F)>,

    func: F,
}

impl<W, A, F> StatelessCallbackFn<W, A, F>
where
    A: Any,
    F: Fn(&mut StatelessCallbackContext, A),
{
    pub fn new(func: F) -> Self {
        Self {
            phantom: PhantomData,

            func,
        }
    }
}

impl<W, A, F> StatelessCallbackFunc<W> for StatelessCallbackFn<W, A, F>
where
    A: Any,
    F: Fn(&mut StatelessCallbackContext, A),
{
    fn call(&self, ctx: &mut StatelessCallbackContext, arg: Box<dyn Any>) {
        let arg = arg
            .downcast::<A>()
            .expect("failed to downcast callback argument");

        (self.func)(ctx, *arg)
    }
}

use std::{any::Any, marker::PhantomData};

use agui_core::unit::AsAny;

use crate::stateful::WidgetState;

use super::StatefulCallbackContext;

pub trait StatefulCallbackFunc<W>
where
    W: WidgetState,
{
    fn call(&self, ctx: &mut StatefulCallbackContext<W>, args: Box<dyn Any>);
}

pub struct StatefulCallbackFn<W, A, F>
where
    A: 'static,
    F: Fn(&mut StatefulCallbackContext<W>, A),
{
    phantom: PhantomData<(W, A, F)>,

    func: F,
}

impl<W, A, F> StatefulCallbackFn<W, A, F>
where
    A: 'static,
    F: Fn(&mut StatefulCallbackContext<W>, A),
{
    pub fn new(func: F) -> Self {
        Self {
            phantom: PhantomData,

            func,
        }
    }
}

impl<W, A, F> StatefulCallbackFunc<W> for StatefulCallbackFn<W, A, F>
where
    W: WidgetState,
    A: AsAny,
    F: Fn(&mut StatefulCallbackContext<W>, A),
{
    fn call(&self, ctx: &mut StatefulCallbackContext<W>, arg: Box<dyn Any>) {
        let arg = arg
            .downcast::<A>()
            .expect("failed to downcast callback argument");

        (self.func)(ctx, *arg)
    }
}

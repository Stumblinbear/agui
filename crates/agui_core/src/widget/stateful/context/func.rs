use std::marker::PhantomData;

use crate::unit::Data;

use super::CallbackContext;

pub trait CallbackFunc<W> {
    #[allow(clippy::borrowed_box)]
    fn call(&self, ctx: &mut CallbackContext<W>, args: Box<dyn Data>);
}

pub struct CallbackFn<W, A, F>
where
    A: 'static,
    F: Fn(&mut CallbackContext<W>, &A),
{
    phantom: PhantomData<(W, A, F)>,

    func: F,
}

impl<W, A, F> CallbackFn<W, A, F>
where
    A: 'static,
    F: Fn(&mut CallbackContext<W>, &A),
{
    pub fn new(func: F) -> Self {
        Self {
            phantom: PhantomData,

            func,
        }
    }
}

impl<W, A, F> CallbackFunc<W> for CallbackFn<W, A, F>
where
    A: Data,
    F: Fn(&mut CallbackContext<W>, &A),
{
    fn call(&self, ctx: &mut CallbackContext<W>, arg: Box<dyn Data>) {
        let arg = arg
            .downcast::<A>()
            .expect("failed to downcast callback argument");

        (self.func)(ctx, arg)
    }
}

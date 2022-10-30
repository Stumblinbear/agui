use std::marker::PhantomData;

use crate::{unit::Data, widget::Widget};

use super::CallbackContext;

pub trait CallbackFunc<W>
where
    W: Widget,
{
    #[allow(clippy::borrowed_box)]
    fn call(&self, ctx: &mut CallbackContext<W>, args: &Box<dyn Data>);
}

pub struct CallbackFn<W, A, F>
where
    W: Widget,
    A: 'static,
    F: Fn(&mut CallbackContext<W>, &A),
{
    phantom: PhantomData<(W, A, F)>,

    func: F,
}

impl<W, A, F> CallbackFn<W, A, F>
where
    W: Widget,
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
    W: Widget,
    A: Data,
    F: Fn(&mut CallbackContext<W>, &A),
{
    fn call(&self, ctx: &mut CallbackContext<W>, args: &Box<dyn Data>) {
        let args = args
            .downcast_ref::<A>()
            .expect("failed to downcast callback args");

        (self.func)(ctx, args)
    }
}

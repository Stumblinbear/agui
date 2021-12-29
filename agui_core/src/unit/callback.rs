use crate::Ref;

type CallbackFn = dyn Fn();

#[derive(Clone)]
pub struct Callback(pub Ref<Box<CallbackFn>>);

impl Default for Callback {
    fn default() -> Self {
        Self(Ref::None)
    }
}

impl<F> From<F> for Callback
where
    F: Fn() + 'static,
{
    fn from(func: F) -> Self {
        Self(Ref::new(Box::new(func)))
    }
}

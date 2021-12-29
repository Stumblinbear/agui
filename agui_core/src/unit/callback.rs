use crate::Ref;

#[derive(Clone)]
pub struct Callback<A>(pub Ref<Box<dyn Fn(A)>>);

impl<A> Default for Callback<A> {
    fn default() -> Self {
        Self(Ref::None)
    }
}

impl<F, A> From<F> for Callback<A>
where
    F: Fn(A) + 'static,
{
    fn from(func: F) -> Self {
        Self(Ref::new(Box::new(func)))
    }
}

impl<A> Callback<A> {
    pub fn emit(&self, arg: A) {
        if let Some(func) = self.0.try_get() {
            func(arg);
        }
    }
}
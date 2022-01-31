use super::Canvas;

pub struct RenderFn<'ui> {
    func: Box<dyn Fn(&mut Canvas) + 'ui>,
}

impl<'ui> RenderFn<'ui> {
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&mut Canvas) + 'ui,
    {
        Self {
            func: Box::new(func),
        }
    }

    pub fn call(&self, canvas: &mut Canvas) {
        (self.func)(canvas);
    }
}

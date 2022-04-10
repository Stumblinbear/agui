use super::Canvas;

pub struct RenderFn {
    func: Box<dyn Fn(&mut Canvas)>,
}

impl RenderFn {
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&mut Canvas) + 'static,
    {
        Self {
            func: Box::new(func),
        }
    }

    pub fn call(&self, canvas: &mut Canvas) {
        (self.func)(canvas);
    }
}

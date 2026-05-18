pub struct TaskHandle {
    drop: Option<Box<dyn FnOnce()>>,
}

impl TaskHandle {
    pub fn new(drop: Box<dyn FnOnce()>) -> Self {
        Self { drop: Some(drop) }
    }
}

impl Drop for TaskHandle {
    fn drop(&mut self) {
        unsafe {
            self.drop.take().unwrap_unchecked()();
        }
    }
}

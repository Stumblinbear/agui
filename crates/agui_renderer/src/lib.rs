use std::error::Error;

pub trait Renderer {
    fn render(&mut self);
}

pub trait BindRenderer<T> {
    #[allow(async_fn_in_trait)]
    async fn bind(
        self,
        target: &T,
        frame_ready: FrameNotifier,
    ) -> Result<Box<dyn Renderer>, Box<dyn Error + Send + Sync>>
    where
        Self: Sized;
}

pub struct FrameNotifier {
    inner: Box<dyn Fn() + Send + Sync>,
}

impl FrameNotifier {
    pub fn new(inner: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    pub fn notify(&self) {
        (self.inner)();
    }
}
